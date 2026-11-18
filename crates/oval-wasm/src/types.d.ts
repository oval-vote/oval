import type { Guide } from "@oval-vote/oval-schema";

/** A compiled guide, the object the specification's schema describes. */
export type { Guide };

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

/** The error a function throws when the document has errors that stop it. */
export interface OvalError extends Error {
    name: "OvalError";
    /** The errors, ordered by where each one begins. */
    diagnostics: Diagnostic[];
}

/** The kind of construct an outline item stands for. */
export type OutlineKind = "ballot" | "section" | "contest" | "measure";

/** One construct in a document's outline, with the constructs nested in it. */
export interface OutlineItem {
    kind: OutlineKind;
    /** The construct's identifier without its `@`, or `null` for a section or when it's missing. */
    id: string | null;
    /** The construct's name or title, or `null` when it's missing. */
    title: string | null;
    /** The whole construct, from its first token through its block. */
    range: Range;
    /** The identifier, a section's title, or the construct's first token when neither is written. */
    selection: Range;
    /** The constructs inside this one, in ballot order. */
    children: OutlineItem[];
}

/** The kind of thing a declaration declares. */
export type Entity = "party" | "candidate" | "ballot" | "contest" | "measure";

/** The declaration an identifier names. */
export interface Definition {
    entity: Entity;
    /** The declared identifier without its `@`. */
    id: string;
    /** The name the declaration gives, the string after its identifier, or `null` when there's none. */
    name: string | null;
    /** The declared identifier, its `@` included. */
    range: Range;
    /** The declared identifier without its `@`. */
    idRange: Range;
}

/** One place an identifier's name is written. */
export interface Occurrence {
    /** The identifier, its `@` included. */
    range: Range;
    /** The identifier without its `@`, which is the text a rename replaces. */
    idRange: Range;
    /** Whether this occurrence declares the name rather than referring to it. */
    isDeclaration: boolean;
}

/** Text describing what's at a position. */
export interface Hover {
    /** The identifier or keyword the text describes. */
    range: Range;
    /** The description in Markdown. */
    contents: string;
}

/** Something the grammar accepts at a position. */
export interface Completion {
    /** The text to insert, which is a keyword or an identifier with its `@`. */
    label: string;
    /** `"keyword"`, or the kind of thing the identifier declares. */
    kind: "keyword" | Entity;
    /** What a keyword does, or the name an identifier declares. */
    detail: string | null;
    /** The text the label replaces, which is the part of the word written before the position. */
    replace: Range;
}

/** A change to a document's text. */
export interface Edit {
    /** The text to replace, which is empty for an insertion. */
    range: Range;
    /** The text to put in its place. */
    text: string;
    /** The part of `text` the author should fill in, as indices into `text`, or `null` when it's complete. */
    placeholder: Range | null;
}

/** A change that repairs one diagnostic. */
export interface Fix {
    /** The position of the diagnostic in `diagnostics()`. */
    diagnostic: number;
    /** What the fix does, in a few words. */
    title: string;
    edit: Edit;
}
