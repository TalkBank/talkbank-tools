#!/usr/bin/env perl
# Fix @ID age fields in corpus/reference/ to conform to CLAN depfile.cut
# patterns: yy; / yy;mm. / yy;mm.dd (two-digit mm and dd).
#
# Context: commit f69746d8 ("CHECK parity") introduced Rust chatter's
# age validation with a bug in AgeValue::needs_zero_padding (early-returned
# on no-period). Fixed 2026-04-21 as E517/violates_depfile_pattern; the
# reference corpus had been coasting on the bug with ages like 3;0 (should
# be 3;00.), 2;6 (should be 2;06.), etc.
#
# Also fixes a single field-7 (SES) placement error in
# languages/ara-conversation.cha where "1;8-1;11Boys" ended up in the
# SES slot.
#
# Usage: perl scripts/fix_corpus_ages.pl  (run from repo root)

use strict;
use warnings;
use File::Find;

my %age_map = (
    '3;0'  => '3;00.',
    '2;0'  => '2;00.',
    '2;6'  => '2;06.',
    '3;6'  => '3;06.',
    '4;0'  => '4;00.',
    '2;06' => '2;06.',
);

my $files_changed = 0;
my $total_subs    = 0;

find(
    sub {
        return unless /\.cha\z/;
        my $path = $File::Find::name;
        open my $fh, '<', $_ or die "open $path: $!";
        my @lines = <$fh>;
        close $fh;

        my $changed = 0;
        for my $line (@lines) {
            # Only touch @ID header lines. Age is field 4 (between the
            # 3rd and 4th pipe); anchor on the exact pipe-bracketed token
            # so we never catch a substring like 2;06.15 as "2;06".
            next unless $line =~ /^\@ID:/;

            for my $bad (keys %age_map) {
                my $good = $age_map{$bad};
                my $n    = $line =~ s/\Q|$bad|\E/|$good|/g;
                if ($n) {
                    $changed    += $n;
                    $total_subs += $n;
                }
            }

            # ara-conversation.cha SES-slot placement fix: empty the
            # nonsense "1;8-1;11Boys" field. It's field 7 (SES), which
            # per depfile.cut should be @s<WC|UC|MC|LI> or empty.
            if ($line =~ s/\|1;8-1;11Boys\|/||/g) {
                $changed    += 1;
                $total_subs += 1;
            }
        }

        if ($changed) {
            open my $out, '>', $_ or die "write $path: $!";
            print {$out} @lines;
            close $out;
            $files_changed += 1;
            printf "  fixed %s (%d substitutions)\n", $path, $changed;
        }
    },
    'corpus/reference'
);

printf "\n%d files updated, %d total substitutions\n",
    $files_changed, $total_subs;
