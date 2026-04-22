#!/usr/bin/env perl
# Fix non-conforming `@ID` age fields in test fixtures and spec files
# across the talkbank-tools tree. Companion to
# `scripts/fix_corpus_ages.pl` (which targets `corpus/reference/`);
# this script covers everything else touched by the E517 validator
# tightening (violates_depfile_pattern, 2026-04-21).
#
# Rule: only the three CLAN `depfile.cut` patterns are legal —
# `yy;`, `yy;mm.`, or `yy;mm.dd` (with two-digit mm/dd). This script
# rewrites any `|Y;M|` / `|Y;MM|` / `|Y;M.D|` / `|Y;MM.D|` / `|Y;M.DD|`
# token it finds to the zero-padded `yy;mm.` or `yy;mm.dd` form.
#
# Scope:
#   crates/**/*.rs     — Rust test/source files with inline CHAT strings
#   crates/**/*.cha    — loose test-fixture CHAT files
#   spec/**/*.md       — error/construct spec files
#
# Leaves `corpus/reference/` alone (handled by the sibling script).
#
# Usage: perl scripts/fix_test_fixture_ages.pl

use strict;
use warnings;
use File::Find;

# Normalize an age string to the nearest legal depfile.cut pattern,
# returning undef if no change is needed or if the input doesn't look
# like a parseable age (so we never mangle unrelated text).
sub normalize_age {
    my ($age) = @_;

    # yy;        — already legal
    return undef if $age =~ /^\d+;$/;
    # yy;mm.     — already legal
    return undef if $age =~ /^\d+;\d{2}\.$/;
    # yy;mm.dd   — already legal
    return undef if $age =~ /^\d+;\d{2}\.\d{2}$/;

    # yy;m       — one-digit month, no period. Zero-pad + add period.
    if ($age =~ /^(\d+);(\d)$/) {
        return sprintf '%s;0%s.', $1, $2;
    }
    # yy;mm      — two-digit month, no period. Add period only.
    if ($age =~ /^(\d+);(\d{2})$/) {
        return sprintf '%s;%s.', $1, $2;
    }
    # yy;m.dd    — one-digit month with day. Pad month.
    if ($age =~ /^(\d+);(\d)\.(\d{2})$/) {
        return sprintf '%s;0%s.%s', $1, $2, $3;
    }
    # yy;mm.d    — two-digit month, one-digit day. Pad day.
    if ($age =~ /^(\d+);(\d{2})\.(\d)$/) {
        return sprintf '%s;%s.0%s', $1, $2, $3;
    }
    # yy;m.d     — both one-digit. Pad both.
    if ($age =~ /^(\d+);(\d)\.(\d)$/) {
        return sprintf '%s;0%s.0%s', $1, $2, $3;
    }
    # yy;.dd or yy;mm. with trailing content — out of scope.
    return undef;
}

my $files_changed = 0;
my $total_subs    = 0;

sub process_file {
    my ($path) = @_;
    # Tolerate broken symlinks (common in target/ and similar).
    return unless -f $path;
    open my $fh, '<', $path or do {
        warn "skipping $path: $!";
        return;
    };
    my $content = do { local $/; <$fh> };
    close $fh;

    # Quick negative filter: skip files with no pipe-delimited age-like
    # token. Avoids spending regex effort on the 90% of source files
    # that never mention a CHAT fragment.
    return unless $content =~ /\|\d+;/;

    my $changed = 0;

    # Match `|<age>|` tokens anywhere in the file. The leading/trailing
    # pipes make this safe against accidental hits inside unrelated
    # numeric content — we only touch strings that look like pipe-
    # delimited age fields.
    $content =~ s{
        \| ( \d+ ; (?: \d{0,2} (?: \. \d{0,2} )? )? ) \|
    }{
        my $orig  = $1;
        my $fixed = normalize_age($orig);
        if (defined $fixed) {
            $changed += 1;
            "|$fixed|"
        } else {
            "|$orig|"
        }
    }gex;

    if ($changed) {
        open my $out, '>', $path or die "write $path: $!";
        print {$out} $content;
        close $out;
        $files_changed += 1;
        $total_subs    += $changed;
        printf "  fixed %s (%d substitutions)\n", $path, $changed;
    }
}

for my $root ('crates', 'spec', 'tests', 'docs', 'grammar') {
    # `no_chdir => 1` keeps the working directory stable so the
    # relative path in `$File::Find::name` stays openable from the
    # repo root. Without it, File::Find chdirs into each subdirectory
    # and the relative path misses.
    find(
        {
            no_chdir => 1,
            wanted   => sub {
                return if -d $File::Find::name;
                return unless $File::Find::name =~ /\.(rs|cha|md)\z/;
                return if $File::Find::name =~ m{/target/};
                return if $File::Find::name =~ m{\.snap(?:\.new)?$};
                return if $File::Find::name =~ m{/generated/};
                return if $File::Find::name =~ m{/experiments/};
                # The E517 spec's purpose is to demonstrate the very
                # age-format violations we'd otherwise "fix" here.
                # Its malformed examples are the positive test cases.
                return if $File::Find::name =~ m{/E517_invalid_age_format\.md$};
                # docs/errors/E517.md is auto-generated from the spec
                # above, so it also contains intentional bad ages.
                return if $File::Find::name =~ m{/docs/errors/E517\.md$};
                process_file($File::Find::name);
            },
        },
        $root
    );
}

printf "\n%d files updated, %d total substitutions\n",
    $files_changed, $total_subs;
