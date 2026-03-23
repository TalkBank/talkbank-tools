# CHAT Error Reference

Complete reference for all CHAT parser and validation errors.

## internal (E0x)

Internal invariant failure. This error indicates a bug in the parseritself, not in the CHAT input. It cannot be triggered by any CHAT file.

| Code | Name | Severity |
|------|------|----------|
| [E001](E001.md) | E001: InternalError | error |

## internal (E0x)

Test-only sentinel error code. Used exclusively in the test suite toverify error handling plumbing. Never emitted in production.

| Code | Name | Severity |
|------|------|----------|
| [E002](E002.md) | E002: TestError | error |

## validation (E0x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E003](E003.md) | generated from corpus | error |

## validation (E1x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E101](E101.md) | generated from corpus | error |

## Parser error (E2x)

Missing form type after @

| Code | Name | Severity |
|------|------|----------|
| [E202](E202.md) | E202: Missing form type after @ | error |

## Word validation (E2x)

A word contains at a position where a form type marker is expected, butno valid form type follows. Tree-sitter produces an ERROR node at the .

| Code | Name | Severity |
|------|------|----------|
| [E202](E202.md) | E202: Missing form type after @ | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E203](E203.md) | generated from corpus | error |

## Word validation (E2x)

Unknown scoped annotation marker

| Code | Name | Severity |
|------|------|----------|
| [E207](E207.md) | E207: Unknown scoped annotation marker | error |

## validation (E2x)

Empty replacement

| Code | Name | Severity |
|------|------|----------|
| [E208](E208.md) | E208: Empty replacement | error |

## validation (E2x)

Empty spoken content

| Code | Name | Severity |
|------|------|----------|
| [E209](E209.md) | E209: Empty spoken content | error |

## Word validation (E2x)

Replacement not allowed for phonological fragment

| Code | Name | Severity |
|------|------|----------|
| [E210](E210.md) | E210: Replacement not allowed for phonological fragment | error |

## Parser error (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E212](E212.md) | generated from corpus | error |

## Word validation (E2x)

Replacement word cannot be untranscribed

| Code | Name | Severity |
|------|------|----------|
| [E213](E213.md) | E213: Replacement word cannot be untranscribed | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E214](E214.md) | generated from corpus | error |

## Word validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E220](E220.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E230](E230.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E231](E231.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E232](E232.md) | generated from corpus | error |

## validation (E2x)

Compound markers () must connect two non-empty parts. Adjacent compound markers() create an empty part between them, which is invalid.

| Code | Name | Severity |
|------|------|----------|
| [E233](E233.md) | E233: Empty compound part | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E241](E241.md) | generated from corpus | error |

## word_validation (E2x)

The marker 'xx' is used for untranscribed speech, but this is not allowed in CHAT. The correct marker for untranscribed speech is 'xxx' (three x's).

| Code | Name | Severity |
|------|------|----------|
| [E241](E241.md) | E241: Illegal Untranscribed Marker 'xx' | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E242](E242.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E243](E243.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E244](E244.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E245](E245.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E246](E246.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E247](E247.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E248](E248.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E249](E249.md) | generated from corpus | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E250](E250.md) | generated from corpus | error |

## validation (E2x)

Empty word content text

| Code | Name | Severity |
|------|------|----------|
| [E251](E251.md) | E251: Empty word content text | error |

## Prosodic marker placement (E2x)

Syntax error - caret at word start

| Code | Name | Severity |
|------|------|----------|
| [E252](E252.md) | caret at word start | error |

## validation (E2x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E253](E253.md) | generated from corpus | error |

## validation (E2x)

Consecutive commas

| Code | Name | Severity |
|------|------|----------|
| [E258](E258.md) | E258: Consecutive commas | error |

## validation (E2x)

Comma without any preceding spoken word in the utterance

| Code | Name | Severity |
|------|------|----------|
| [E259](E259.md) | spoken content | error |

## Main tier validation (E3x)

Empty speaker code

| Code | Name | Severity |
|------|------|----------|
| [E301](E301.md) | E301: Empty speaker code | error |

## validation (E3x)

Missing required node

| Code | Name | Severity |
|------|------|----------|
| [E302](E302.md) | E302: Missing required node | error |

## Parser bugs (experimental) (E3x)

Unexpected node - helper function

| Code | Name | Severity |
|------|------|----------|
| [E303](E303.md) | helper function | error |

## Main tier validation (E3x)

Expected terminator not found

| Code | Name | Severity |
|------|------|----------|
| [E304](E304.md) | E304: Expected terminator not found | error |

## Main tier validation (E3x)

Expected main tier content

| Code | Name | Severity |
|------|------|----------|
| [E305](E305.md) | E305: Expected main tier content | error |

## Main tier validation (E3x)

Utterance has no content

| Code | Name | Severity |
|------|------|----------|
| [E306](E306.md) | E306: Utterance has no content | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E307](E307.md) | generated from corpus | error |

## Main tier validation (E3x)

Invalid speaker format

| Code | Name | Severity |
|------|------|----------|
| [E308](E308.md) | E308: Invalid speaker format | error |

## validation (E3x)

Unexpected syntax

| Code | Name | Severity |
|------|------|----------|
| [E309](E309.md) | E309: Unexpected syntax | error |

## Main tier validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E310](E310.md) | generated from corpus | error |

## Main tier validation (E3x)

Failed to parse utterance

| Code | Name | Severity |
|------|------|----------|
| [E311](E311.md) | E311: Failed to parse utterance | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E312](E312.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E313](E313.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E314](E314.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E315](E315.md) | generated from corpus | error |

## Main tier validation (E3x)

Unparsable content

| Code | Name | Severity |
|------|------|----------|
| [E316](E316.md) | E316: Unparsable content | error |

## parser_recovery (E3x)

A line could not be classified as a header, utterance, or dependent tier.This is a fallback error emitted when tree-sitter produces an ERROR nodefor a line whose children cannot be identified as either a header orutterance context.

| Code | Name | Severity |
|------|------|----------|
| [E319](E319.md) | E319: UnparsableLine | error |

## parser_recovery (E3x)

A header line (starting with @) could not be parsed. This is a fallbackerror emitted when tree-sitter produces an ERROR node in header context,but the header type is not one of the specifically handled types(@Participants, @Languages, @Date, @Media, @ID).

| Code | Name | Severity |
|------|------|----------|
| [E320](E320.md) | E320: UnparsableHeader | error |

## parser_recovery (E3x)

An utterance line (starting with *SPEAKER:) could not be parsed. Theutterance body contains syntax errors that tree-sitter cannot recoverfrom, and the error doesn't match any of the specifically checkedpatterns (missing form type, empty replacement, unknown annotation).

| Code | Name | Severity |
|------|------|----------|
| [E321](E321.md) | E321: UnparsableUtterance | error |

## parser_recovery (E3x)

The main tier speaker prefix has a zero-width (MISSING) colon node.This occurs when tree-sitter synthesizes an empty colon placeholderbecause the speaker code has no colon at all.

| Code | Name | Severity |
|------|------|----------|
| [E322](E322.md) | E322: EmptyColon | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E323](E323.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E324](E324.md) | generated from corpus | error |

## parser_recovery (E3x)

An unexpected child node was found inside a parsed utterance. The CSTcontains a node that is neither the main tier nor a recognized dependenttier kind. This typically indicates a tree-sitter error recovery scenariowhere an unusual node type ends up inside an utterance subtree.

| Code | Name | Severity |
|------|------|----------|
| [E325](E325.md) | E325: UnexpectedUtteranceChild | error |

## parser_recovery (E3x)

A line was classified as an unexpected type during file structure parsing.This covers two sub-cases:

| Code | Name | Severity |
|------|------|----------|
| [E326](E326.md) | E326: UnexpectedLineType | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E330](E330.md) | generated from corpus | error |

## parser_recovery (E3x)

A tree-sitter node appeared in a syntactic context where it is not expected. The nodetype itself is valid CHAT syntax, but it occurs at a position in the AST that violatesthe grammar. This error is emitted during tree-sitter error recovery — the parserattempts to continue after encountering invalid syntax, and the recovered structurecontains nodes in unexpected positions.

| Code | Name | Severity |
|------|------|----------|
| [E331](E331.md) | E331: UnexpectedNodeInContext | error |

## parser_recovery (E3x)

Main tier content could not be classified as any known word or constructtype. This fires when a CST node has a child kindthat the Rust parser doesn't recognize — indicating a grammar/parsermismatch (the grammar produces a new node type that the parser hasn'tbeen updated to handle).

| Code | Name | Severity |
|------|------|----------|
| [E340](E340.md) | E340: UnknownBaseContent | error |

## cross_utterance (E3x)

A quotation-follows terminator () is used but the next utterancefrom the same speaker does not begin with a quotation precedes linker(). This indicates an unbalanced cross-utterance quotation sequence.

| Code | Name | Severity |
|------|------|----------|
| [E341](E341.md) | E341: UnbalancedQuotationCrossUtterance | error |

## Word validation (E3x)

Missing required element

| Code | Name | Severity |
|------|------|----------|
| [E342](E342.md) | E342: Missing required element | error |

## validation (E3x)

Invalid scoped annotation nesting

| Code | Name | Severity |
|------|------|----------|
| [E344](E344.md) | E344: Invalid scoped annotation nesting | error |

## validation (E3x)

Unmatched scoped annotation begin

| Code | Name | Severity |
|------|------|----------|
| [E345](E345.md) | E345: Unmatched scoped annotation begin | error |

## validation (E3x)

Unmatched scoped annotation end

| Code | Name | Severity |
|------|------|----------|
| [E346](E346.md) | E346: Unmatched scoped annotation end | error |

## validation (E3x)

An indexed top overlap region (⌈2...⌉2) on one speaker has no matchingindexed bottom overlap region (⌊2...⌋2) from a different speaker within thenearby utterances, or vice versa. Reported as a warning.

| Code | Name | Severity |
|------|------|----------|
| [E347](E347.md) | speaker overlap | error |

## validation (E3x)

A closing overlap marker (⌉ or ⌋) appears without a preceding opening marker(⌈ or ⌊) within the same utterance. Reported as a warning.

| Code | Name | Severity |
|------|------|----------|
| [E348](E348.md) | E348: Unpaired overlap marker within utterance | error |

## cross_utterance (E3x)

A self-completion linker () was used but there is no prior utterancefrom the same speaker. The linker requires a preceding interruptedutterance from the same speaker to complete.

| Code | Name | Severity |
|------|------|----------|
| [E351](E351.md) | E351: MissingQuoteBegin | error |

## cross_utterance (E3x)

A self-completion linker () was used and there IS a prior utterancefrom the same speaker, but that prior utterance did not end with a (interruption) terminator.

| Code | Name | Severity |
|------|------|----------|
| [E352](E352.md) | E352: MissingQuoteEnd | error |

## cross_utterance (E3x)

An other-completion linker () was used but it is the very firstutterance in the file. The linker requires a preceding utterance(from a different speaker) to complete.

| Code | Name | Severity |
|------|------|----------|
| [E353](E353.md) | E353: MissingOtherCompletionContext | error |

## cross_utterance (E3x)

An other-completion linker () was used and the preceding utterance isfrom a different speaker, but that preceding utterance did not end with (trailing off). The other-completion convention requires theprevious speaker to have trailed off.

| Code | Name | Severity |
|------|------|----------|
| [E354](E354.md) | E354: MissingTrailingOffTerminator | error |

## cross_utterance (E3x)

An other-completion linker () was used but the preceding utterance isfrom the same speaker. The linker is for other-completion(completing a different speaker's utterance). To complete one's ownutterance, use (self-completion) instead.

| Code | Name | Severity |
|------|------|----------|
| [E355](E355.md) | E355: InterleavedScopedAnnotations | error |

## underline_balance (E3x)

An underline begin marker was found without a matching underline end markerin the same utterance. Underline markers (used in CA transcription to markstressed syllables) must occur in matched begin/end pairs within a singleutterance.

| Code | Name | Severity |
|------|------|----------|
| [E356](E356.md) | E356: UnmatchedUnderlineBegin | error |

## underline_balance (E3x)

An underline end marker was found without a preceding underline beginmarker in the same utterance. The end marker has no open underline toclose.

| Code | Name | Severity |
|------|------|----------|
| [E357](E357.md) | E357: UnmatchedUnderlineEnd | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E358](E358.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E359](E359.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E360](E360.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E361](E361.md) | generated from corpus | error |

## validation (E3x)

Bullet timestamps must be monotonic

| Code | Name | Severity |
|------|------|----------|
| [E362](E362.md) | E362: Bullet timestamps must be monotonic | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E363](E363.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E364](E364.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E365](E365.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E367](E367.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E368](E368.md) | generated from corpus | error |

## Alignment count mismatch (E3x)

Structural order error

| Code | Name | Severity |
|------|------|----------|
| [E370](E370.md) | E370: Structural order error | error |

## validation (E3x)

Pause inside phonological group

| Code | Name | Severity |
|------|------|----------|
| [E371](E371.md) | E371: Pause inside phonological group | error |

## validation (E3x)

Nested quotation

| Code | Name | Severity |
|------|------|----------|
| [E372](E372.md) | E372: Nested quotation | error |

## overlap (E3x)

An overlap marker has an index value outside the valid range. For CAoverlap brackets (), the index must be 2–9. For scoped overlapannotations (, ), the index must be 1–9.

| Code | Name | Severity |
|------|------|----------|
| [E373](E373.md) | E373: InvalidOverlapIndex | error |

## Parser bugs (experimental) (E3x)

Scoped annotation parse error

| Code | Name | Severity |
|------|------|----------|
| [E375](E375.md) | E375: Scoped annotation parse error | error |

## Word validation (E3x)

Failed to parse replacement annotation content. The replacementannotation contains content that cannot be parsed as valid replacementwords.

| Code | Name | Severity |
|------|------|----------|
| [E376](E376.md) | E376: Replacement parse error | error |

## tier_parse (E3x)

The tier content could not be parsed by the direct parser (chumsky).The phonological tier content does not match the expected format ofspace-separated phonological words with optional groups.

| Code | Name | Severity |
|------|------|----------|
| [E381](E381.md) | E381: PhoParseError | error |

## Dependent tier parsing (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E382](E382.md) | generated from corpus | error |

## Dependent tier parsing (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E383](E383.md) | generated from corpus | error |

## tier_parse (E3x)

The tier content could not be parsed by the direct parser (chumsky).The sentence-internal tier content does not match the expected format.

| Code | Name | Severity |
|------|------|----------|
| [E384](E384.md) | E384: SinParseError | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E387](E387.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E388](E388.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E389](E389.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E390](E390.md) | generated from corpus | error |

## validation (E3x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E391](E391.md) | generated from corpus | error |

## validation (E4x)

Duplicate dependent tiers

| Code | Name | Severity |
|------|------|----------|
| [E401](E401.md) | E401: Duplicate dependent tiers | error |

## validation (E4x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E404](E404.md) | generated from corpus | error |

## Header validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E501](E501.md) | generated from corpus | error |

## validation (E5x)

Every valid CHAT file must end with an header. This error indicates the file is missing , usually because the file is truncated, empty, or was saved incompletely.

| Code | Name | Severity |
|------|------|----------|
| [E502](E502.md) | E502: Missing required @End header | error |

## parser (E5x)

When a tier contains invalid content (e.g., an action marker like )AND the %wor line has 7+ words after the error, tree-sitter's error recovery failscatastrophically: instead of isolating the ERROR to the %wor tier, the entire filebecomes one ERROR node. This causes:

| Code | Name | Severity |
|------|------|----------|
| [E502 (false positive)](E502 (false positive).md) | E502 false positive: %wor parse error cascades to entire file | error |

## Header validation (E5x)

Every valid CHAT file must begin with an header as its first line. This error indicates the file is missing , which means the file's character encoding is unspecified. All modern CHAT files are expected to be UTF-8 encoded.

| Code | Name | Severity |
|------|------|----------|
| [E503](E503.md) | E503: Missing required @UTF8 header | error |

## Header validation (E5x)

Missing required header

| Code | Name | Severity |
|------|------|----------|
| [E504](E504.md) | E504: Missing required header | error |

## Header validation (E5x)

Invalid @ID format

| Code | Name | Severity |
|------|------|----------|
| [E505](E505.md) | E505: Invalid @ID format | error |

## Header validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E506](E506.md) | generated from corpus | error |

## Header validation (E5x)

@Languages header cannot be empty

| Code | Name | Severity |
|------|------|----------|
| [E507](E507.md) | E507: @Languages header cannot be empty | error |

## Header validation (E5x)

@Date header cannot be empty

| Code | Name | Severity |
|------|------|----------|
| [E508](E508.md) | E508: @Date header cannot be empty | error |

## Header validation (E5x)

@Media header cannot be empty

| Code | Name | Severity |
|------|------|----------|
| [E509](E509.md) | E509: @Media header cannot be empty | error |

## Header validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E510](E510.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E511](E511.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E512](E512.md) | generated from corpus | error |

## Header validation (E5x)

Participant entry should have both code and role

| Code | Name | Severity |
|------|------|----------|
| [E513](E513.md) | E513: Participant entry should have both code and role | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E515](E515.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E516](E516.md) | generated from corpus | error |

## Header validation - Age format (E5x)

Age should be in format years;months.days

| Code | Name | Severity |
|------|------|----------|
| [E517](E517.md) | E517: Age should be in format years;months.days | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E518](E518.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E519](E519.md) | generated from corpus | error |

## Header validation (E5x)

@Participants header cannot be empty

| Code | Name | Severity |
|------|------|----------|
| [E522](E522.md) | E522: @Participants header cannot be empty | error |

## header_validation (E5x)

An utterance uses a speaker code that was not defined in the @Participants header. All speaker codes used in utterances must be declared in the @Participants header.

| Code | Name | Severity |
|------|------|----------|
| [E522](E522.md) | E522: Undefined Participant in Utterance | error |

## Participant validation (E5x)

Orphan @ID header

| Code | Name | Severity |
|------|------|----------|
| [E523](E523.md) | E523: Orphan @ID header | error |

## Participant validation (E5x)

@Birth header for unknown participant

| Code | Name | Severity |
|------|------|----------|
| [E524](E524.md) | E524: @Birth header for unknown participant | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E525](E525.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E526](E526.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E527](E527.md) | generated from corpus | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E528](E528.md) | generated from corpus | error |

## validation (E5x)

Nested background with identical label

| Code | Name | Severity |
|------|------|----------|
| [E529](E529.md) | E529: Nested background with identical label | error |

## validation (E5x)

Lazy gem inside background

| Code | Name | Severity |
|------|------|----------|
| [E530](E530.md) | E530: Lazy gem inside background | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E531](E531.md) | generated from corpus | error |

## validation (E5x)

Invalid participant role

| Code | Name | Severity |
|------|------|----------|
| [E532](E532.md) | E532: Invalid participant role | error |

## validation (E5x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E533](E533.md) | generated from corpus | error |

## header_validation (E5x)

An header contains a flag that is not one of the recognized option values. The file parses successfully but the unsupported flag is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E534](E534.md) | E534: Unsupported @Options Value | error |

## header_validation (E5x)

An header contains a media type that is not one of the recognized values. The file parses successfully but the unsupported type is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E535](E535.md) | E535: Unsupported @Media Type | error |

## header_validation (E5x)

An header contains a status value that is not one of the recognized values. The file parses successfully but the unsupported status is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E536](E536.md) | E536: Unsupported @Media Status | error |

## header_validation (E5x)

An header contains a value that is not one of the recognized number options. The file parses successfully but the unsupported value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E537](E537.md) | E537: Unsupported @Number Value | error |

## header_validation (E5x)

An header contains a value that is not one of the recognized quality ratings. The file parses successfully but the unsupported value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E538](E538.md) | E538: Unsupported @Recording Quality Value | error |

## header_validation (E5x)

An header contains a value that is not one of the recognized transcription types. The file parses successfully but the unsupported value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E539](E539.md) | E539: Unsupported @Transcription Value | error |

## header_validation (E5x)

An header contains a value that does not match the expected time format. The file parses successfully but the invalid value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E540](E540.md) | E540: Invalid @Time Duration Format | error |

## header_validation (E5x)

An header contains a value that does not match the expected time format. The file parses successfully but the invalid value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E541](E541.md) | E541: Invalid @Time Start Format | error |

## header_validation (E5x)

An header contains a sex field value that is not one of the recognized values. The file parses successfully but the unsupported value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E542](E542.md) | E542: Unsupported @ID Sex Value | error |

## header_validation (E5x)

A header appears out of canonical order. For example, or appears before . CHAT headers must follow the canonical ordering: , , , , then other headers like and .

| Code | Name | Severity |
|------|------|----------|
| [E543](E543.md) | E543: Header out of canonical order | error |

## header_validation (E5x)

An header contains an SES (socioeconomic status) field value that is not one of the recognized values. The file parses successfully but the unsupported value is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E546](E546.md) | E546: Unsupported @ID SES Value | error |

## validation (E6x)

A dependent tier (typically ) had parse errors during lenient recovery, so thevalidator cannot verify alignment between tiers. Alignment checks (main↔%mor, %mor↔%gra)are skipped for the affected utterance. This is a warning, not an error — the filestill parses, but alignment correctness is unverified for tainted tiers.

| Code | Name | Severity |
|------|------|----------|
| [E600](E600.md) | E600: Tier alignment skipped due to parse errors | error |

## validation (E6x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E601](E601.md) | generated from corpus | error |

## validation (E6x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E602](E602.md) | generated from corpus | error |

## tier_validation (E6x)

A dependent tier contains content that does not match the expected time format. The tier parses successfully but the invalid content is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E603](E603.md) | E603: Invalid %tim Tier Format | error |

## Dependent tier parsing (E6x)

Empty GRA relation

| Code | Name | Severity |
|------|------|----------|
| [E604](E604.md) | E604: Empty GRA relation | error |

## tier_validation (E6x)

A %gra (grammatical relations) tier appears without a corresponding %mor (morphology) tier. According to CHAT rules, %gra depends on %mor and cannot exist independently.

| Code | Name | Severity |
|------|------|----------|
| [E604](E604.md) | E604: %gra Tier Without %mor Tier | error |

## tier_validation (E6x)

An utterance contains a dependent tier with a label that is not a standard CHAT tier name and does not follow the user-defined tier naming convention. The file parses successfully but the tier is stored as and flagged during validation.

| Code | Name | Severity |
|------|------|----------|
| [E605](E605.md) | E605: Unsupported Dependent Tier | error |

## Dependent tier parsing (E7x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E701](E701.md) | generated from corpus | error |

## Dependent tier parsing (E7x)

Invalid MOR chunk format - missing |

| Code | Name | Severity |
|------|------|----------|
| [E702](E702.md) | missing | | error |

## validation (E7x)

A single speaker has consecutive utterances with overlap markers (/) that overlap with each other. Overlap markers should indicate simultaneous speech between different speakers, not self-overlap.

| Code | Name | Severity |
|------|------|----------|
| [E704](E704.md) | overlap — overlapping overlap markers | error |

## Alignment count mismatch (E7x)

Mor count mismatch - too few items

| Code | Name | Severity |
|------|------|----------|
| [E705](E705.md) | too few items | error |

## Alignment count mismatch (E7x)

Mor count mismatch - too many mor items

| Code | Name | Severity |
|------|------|----------|
| [E706](E706.md) | too many mor items | error |

## Alignment terminator mismatch (E7x)

The %mor tier has a terminator but the main tier does not, or vice versa.One tier ends with a sentence-final punctuation mark while the other does not.

| Code | Name | Severity |
|------|------|----------|
| [E707](E707.md) | E707: Mor terminator presence mismatch | error |

## Dependent tier parsing (E7x)

GRA relation missing index

| Code | Name | Severity |
|------|------|----------|
| [E708](E708.md) | E708: GRA relation missing index | error |

## validation (E7x)

Invalid grammar index

| Code | Name | Severity |
|------|------|----------|
| [E709](E709.md) | E709: Invalid grammar index | error |

## Dependent tier parsing (E7x)

Invalid GRA format

| Code | Name | Severity |
|------|------|----------|
| [E710](E710.md) | E710: Invalid GRA format | error |

## Mor content validation (E7x)

A %mor word has an empty stem, POS category, prefix, or suffix. Everymorphosyntax item on the %mor tier must have a non-empty POS categoryand a non-empty stem at minimum.

| Code | Name | Severity |
|------|------|----------|
| [E711](E711.md) | E711: Mor empty content | error |

## validation (E7x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [E712](E712.md) | generated from corpus | error |

## Alignment count mismatch (E7x)

Gra head index invalid

| Code | Name | Severity |
|------|------|----------|
| [E713](E713.md) | E713: Gra head index invalid | error |

## Alignment count mismatch (E7x)

Pho count mismatch - too few pho tokens

| Code | Name | Severity |
|------|------|----------|
| [E714](E714.md) | too few pho tokens | error |

## Alignment count mismatch (E7x)

Pho count mismatch - too many pho tokens

| Code | Name | Severity |
|------|------|----------|
| [E715](E715.md) | too many pho tokens | error |

## Alignment terminator mismatch (E7x)

The %mor tier has a terminator that does not match the main tier's terminator.Both tiers have terminators, but they differ (e.g., main tier ends with "?"but %mor ends with "."). This typically indicates stale or incorrectly cachedmorphosyntax data.

| Code | Name | Severity |
|------|------|----------|
| [E716](E716.md) | E716: Mor terminator value mismatch | error |

## Alignment count mismatch (E7x)

Sin count mismatch - too few sin tokens

| Code | Name | Severity |
|------|------|----------|
| [E718](E718.md) | too few sin tokens | error |

## Alignment count mismatch (E7x)

Sin count mismatch - too many sin tokens

| Code | Name | Severity |
|------|------|----------|
| [E719](E719.md) | too many sin tokens | error |

## Alignment count mismatch (E7x)

Mor-Gra count mismatch

| Code | Name | Severity |
|------|------|----------|
| [E720](E720.md) | Gra count mismatch | error |

## validation (E7x)

GRA non-sequential index

| Code | Name | Severity |
|------|------|----------|
| [E721](E721.md) | sequential index | error |

## validation (E7x)

GRA has no ROOT

| Code | Name | Severity |
|------|------|----------|
| [E722](E722.md) | E722: GRA has no ROOT | error |

## validation (E7x)

GRA has multiple ROOTs

| Code | Name | Severity |
|------|------|----------|
| [E723](E723.md) | E723: GRA has multiple ROOTs | error |

## validation (E7x)

A %gra tier contains a circular dependency where following parent pointers creates a cycle. This violates the fundamental requirement that dependency structures must form a tree.

| Code | Name | Severity |
|------|------|----------|
| [E724](E724.md) | E724: GRA has circular dependency | error |

## Alignment count mismatch (E7x)

The tier word count does not match the tier word count. Each word-level entry in must correspond one-to-one with a word-level entry in .

| Code | Name | Severity |
|------|------|----------|
| [E725](E725.md) | E725: Modsyl tier word count does not match mod tier | error |

## Alignment count mismatch (E7x)

The tier word count does not match the tier word count. Each word-level entry in must correspond one-to-one with a word-level entry in .

| Code | Name | Severity |
|------|------|----------|
| [E726](E726.md) | E726: Phosyl tier word count does not match pho tier | error |

## Alignment count mismatch (E7x)

The tier word count does not match the tier word count. Each word-level entry in must correspond one-to-one with a word-level entry in .

| Code | Name | Severity |
|------|------|----------|
| [E727](E727.md) | E727: Phoaln tier word count does not match mod tier | error |

## Alignment count mismatch (E7x)

The tier word count does not match the tier word count. Each word-level entry in must correspond one-to-one with a word-level entry in .

| Code | Name | Severity |
|------|------|----------|
| [E728](E728.md) | E728: Phoaln tier word count does not match pho tier | error |

## Alignment count mismatch (E9x)

Unknown error

| Code | Name | Severity |
|------|------|----------|
| [E999](E999.md) | E999: Unknown error | error |

## validation (W1x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [W108](W108.md) | generated from corpus | error |

## Warnings (W6x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [W601](W601.md) | generated from corpus | error |

## Warnings (W6x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [W602](W602.md) | generated from corpus | error |

## validation (W7x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [W724](W724.md) | generated from corpus | error |

## validation (W9x)

Auto-generated from corpus

| Code | Name | Severity |
|------|------|----------|
| [W999](W999.md) | generated from corpus | error |

