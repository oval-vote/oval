/** A span of a document, from `start` up to `end`, as string indices. */
export interface Range {
    start: number;
    end: number;
}

/** The identifier the specification gives a problem. */
export type DiagnosticCode =
    | "illegal-tab"
    | "unexpected-character"
    | "unterminated-string"
    | "invalid-escape"
    | "malformed-number"
    | "malformed-url"
    | "inconsistent-dedent"
    | "unexpected-token"
    | "missing-identifier"
    | "missing-name"
    | "missing-contest-kind"
    | "missing-value"
    | "missing-body"
    | "missing-block"
    | "unexpected-block"
    | "unexpected-entry"
    | "repeated-field"
    | "repeated-flag"
    | "repeated-modifier"
    | "misplaced-entry"
    | "empty-recommendation"
    | "unexpected-stance"
    | "unsupported-version"
    | "missing-guide-field"
    | "empty-document"
    | "invalid-date"
    | "undeclared-reference"
    | "duplicate-declaration"
    | "mismatched-reference"
    | "block-on-reference"
    | "misplaced-candidate-field"
    | "empty-ballot"
    | "empty-section"
    | "contest-without-campaigns"
    | "campaign-without-members"
    | "duplicate-member"
    | "member-in-two-campaigns"
    | "party-in-two-campaigns"
    | "invalid-multiple-count"
    | "invalid-ranked-max"
    | "invalid-seat-count"
    | "invalid-minimum"
    | "integer-too-large"
    | "ranked-all-with-limit"
    | "retention-campaign-count"
    | "retention-party-campaign"
    | "missing-recommendation"
    | "repeated-recommendation"
    | "campaign-not-in-contest"
    | "repeated-oppose"
    | "repeated-stance"
    | "tiers-in-single"
    | "support-over-max"
    | "retention-stance"
    | "abstain-on-required"
    | "write-in-not-accepted"
    | "extensible-retention"
    | "markdown-profile"
    | "unused-declaration"
    | "duplicate-party-name"
    | "duplicate-ballot-title"
    | "support-over-count"
    | "support-below-minimum"
    | "write-in-on-ballot"
    | "unpaired-outcome"
    | "insecure-url"
    | "empty-string";

/** How serious a diagnostic is. */
export type Severity = "error" | "warning";

/** A problem found in a document. */
export interface Diagnostic {
    code: DiagnosticCode;
    severity: Severity;
    /** The text the problem spans, which is empty where a missing token would begin. */
    range: Range;
    /** A sentence describing the problem for a person to read. */
    message: string;
}
