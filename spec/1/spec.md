# Oval Vote format, version 1

Status: draft. Sections 3 through 7 are the phase 1 deliverable and are complete
enough to implement against. The other sections are outlined and will be written
before the grammar freezes.

The key words MUST, MUST NOT, REQUIRED, SHALL, SHOULD, SHOULD NOT, RECOMMENDED,
MAY, and OPTIONAL are to be interpreted as described in RFC 2119 and RFC 8174
when, and only when, they appear in all capitals.

## 1. Scope and goals

*Outline.* A `.oval` file is one author's voting guide for one election: the
contests and measures on the ballot, the author's positions, and the reasoning.
The format is text, readable without tooling, diffable, and designed for a
lossless parser so that editors can offer an outline, diagnostics, completion,
and formatting.

## 2. Conformance and terminology

*Outline.* Defines: file, document, section, region, contest, measure, retention,
candidate, id, kind line, field, rationale, conforming parser, conforming
producer, conforming formatter.

## 3. Lexical structure

### 3.1 Encoding

A file MUST be UTF-8. A producer MUST NOT write a byte order mark. A parser
SHOULD accept a leading byte order mark and MUST report it as a diagnostic.

### 3.2 Lines

A file is a sequence of lines. A line ends with LF or CRLF; a parser MUST accept
either and MUST NOT distinguish them. A producer MUST write LF. The last line
MAY lack a terminator.

Trailing whitespace on a line is not significant. A *blank line* is a line that
is empty or contains only whitespace.

The grammar in section 4 is line-oriented: every construct begins at the start
of a line and the first characters of a line decide what the line is.

### 3.3 Indentation

A line beginning with one or more spaces is a *continuation line*. A tab at the
start of a line is an error. The amount of indentation is not significant;
producers SHOULD use two spaces. Which construct a continuation line continues
is defined where each construct is described.

### 3.4 Comments

A line beginning with `//` at column one is a comment. Comments carry no meaning
in the data model. A formatter MUST preserve them in place. A comment line
inside a paragraph does not end the paragraph.

### 3.5 Identifiers

```
id = ( ALPHA | DIGIT ) *( ALPHA | DIGIT | "." | "_" | "-" )
```

Identifiers are case-sensitive and compared byte-wise. An identifier in a
heading or candidate line is introduced by `@`. Where a construct has no
explicit identifier, one is *derived* (section 6.1).

### 3.6 Text

Everything the grammar calls `text` is a run of characters to the end of the
line, with leading and trailing whitespace removed. Inline markup (section 7)
applies only to the text of rationale paragraphs, descriptions, notes, and
candidate notes. Names, field values other than notes, and tags are plain text.

## 4. Grammar

The formal grammar is in `grammar.ebnf`. This section describes each construct.

### 4.1 Magic line

The first line of a file MUST be `oval` followed by one space and the major
version as a decimal integer. For this specification the line is exactly:

```
oval 1
```

A parser MUST reject a file whose major version it does not implement. There is
no other way to identify a file as Oval Vote; the magic line is also what
`file(1)` and editors use.

### 4.2 Header

The header is every line between the magic line and the first heading. It
consists of fields (section 4.6), comments, and blank lines.

| Key        | Required | Value                                                  |
|------------|----------|--------------------------------------------------------|
| `name`     | yes      | The guide's title, e.g. `2026 General Election`.       |
| `date`     | yes      | Election day as an RFC 3339 full-date, `YYYY-MM-DD`.   |
| `author`   | yes      | `Name` or `Name <url>`. The url is an absolute http(s) URL in angle brackets at the end of the value. |
| `updated`  | no       | A full-date; when the guide was last revised.          |
| `language` | no       | A BCP 47 language tag for the rationale text. Default `en`. |

A key MAY appear at most once. Unknown keys are errors unless they begin with
`x-` (section 8).

### 4.3 Headings

```
heading = 1*6"#" SP text [ SP "@" id ]
```

A heading starts a *section*. The number of `#` characters is its *level*.
Sections nest by level exactly as in Markdown: a heading of level n is a child
of the nearest preceding heading of level less than n, and a top-level heading
(level 1) is a child of the document.

The first heading in a file MUST be level 1. A heading's level MUST NOT exceed
its parent's level plus one. Closing `#` characters are not supported.

If the heading's last whitespace-separated token begins with `@`, it is the
section's identifier and is not part of the name. The remaining text is the
section's *name*. The name MUST NOT be empty.

### 4.4 Description

Immediately after a heading, before anything but blank lines and comments, a
section MAY have a description: one or more consecutive lines beginning with
`>`. The text after `>` on each line is joined with single spaces into one
paragraph. The description MAY contain inline markup. It is a short summary
shown beside the name in outlines and indexes; longer explanation belongs in a
note or in rationale.

### 4.5 Kind line

The first line after the heading and description that is not blank and not a
comment decides what the section is. If that line begins with one of the kind
keywords, it is the *kind line*:

```
kindline = kind *( "," *SP tag )
kind     = "region" | "contest" | "measure" | "retention"
```

A section with no kind line is a plain *section*: a container for other
sections. A `region` is also a container, marked as a place (a country, state,
county, city, district) so that viewers can build breadcrumbs and let readers
filter by where they vote. Containers MUST NOT have fields, candidates, or
rationale.

A `contest`, `measure`, or `retention` is a *question*: something the reader
votes on. Questions MUST NOT contain sections: a heading with a deeper level
than a question's is an error.

Tags qualify the kind. A tag MUST NOT appear twice on one line. Unknown tags are
errors unless they begin with `x-`.

Tags for `contest`:

| Tag             | Meaning                                                   |
|-----------------|-----------------------------------------------------------|
| `partisan`      | The ballot prints a party beside each candidate.          |
| `ranked`        | Ranked-choice voting. Picks are numbered (section 4.7).   |
| `N seats`       | Vote for up to N candidates, N ≥ 2. Mutually exclusive with `ranked`. |
| `rank up to N`  | The ballot allows at most N rankings. Requires `ranked`.  |
| `write-ins`     | The ballot has a write-in line.                           |
| `unverified`    | The candidate list has not been checked against an official ballot. |

Without `ranked` or `N seats` a contest is vote-for-one.

Tags for `measure` and `retention`: `unverified`, with the same meaning.

Tags for `region`: none.

### 4.6 Fields

```
field        = key ":" [ SP text ]
continuation = 1*SP text
key          = word *( SP word )
word         = LCALPHA *( LCALPHA | "-" LCALPHA )
```

A field is a lowercase key (words of letters and single hyphens, separated by
single spaces), a colon, and a value. Continuation lines extend the
value; the lines are joined with single spaces. A blank line ends the field. A
field with an empty value is the same as an absent field.

Fields MAY appear anywhere in a question's structure block (section 4.8), in any
order. A key MAY appear at most once per section. Which keys are allowed
depends on the kind:

| Key           | Kinds                  | Value                                    |
|---------------|------------------------|------------------------------------------|
| `note`        | contest, measure, retention | Context the ballot does not carry. Inline markup. |
| `vote`        | contest                | `undecided` or `none`. Only when no candidate is picked or ranked. |
| `vote`        | measure, retention     | `yes`, `no`, `undecided`, or `none`. REQUIRED. |
| `party`       | contest                | For a party primary: the party whose nomination is contested. |
| `placed by`   | measure                | Who put the measure on the ballot. Plain text. |
| `yes`         | measure                | What a yes vote does. Plain text.         |
| `no`          | measure                | What a no vote does. Plain text.          |
| `threshold`   | measure                | The share of votes needed to pass, e.g. `two-thirds`. Plain text. |
| `supporters`  | measure                | Comma-separated list of names.           |
| `opponents`   | measure                | Comma-separated list of names.           |
| `designation` | retention              | The office held, e.g. `Associate Justice`. |

List-valued fields split the value on commas and trim each item; an item MUST
NOT be empty.

### 4.7 Candidate lines

```
candidate = marker SP name [ SP "(" labels ")" ] [ ":" SP text ] [ SP "@" id ]
marker    = "+" | "-" | "*" | 1*DIGIT "."
labels    = label *( "," *SP label )
```

Candidate lines are allowed only in contests and list every candidate on the
ballot for that contest. The marker is the author's position on the candidate:

| Marker | Meaning                                                            |
|--------|--------------------------------------------------------------------|
| `+`    | Pick. Vote for this candidate. Not allowed in `ranked` contests.   |
| `N.`   | Rank N. Only in `ranked` contests.                                 |
| `*`    | Listed without a position.                                         |
| `-`    | Avoid. Do not vote for this candidate; in a ranked contest do not rank them. |

The *name* runs to the first ` (`, `:`, or ` @` that begins a later part, or to
the end of the line. Labels are the ballot's words about the candidate: a party
name, `incumbent`, or `write-in`. At most one label may be something other than
`incumbent` or `write-in`, and it is the party. A `write-in` candidate is not
printed on the ballot; the name is what the voter writes, and the contest MUST
carry the `write-ins` tag.

The text after `:` is the *designation*: a few words saying who the candidate
is, usually the ballot designation as printed. Use the `incumbent` label rather
than writing it here.

Continuation lines after a candidate line are the candidate's *note*, one
paragraph with inline markup.

In a ranked contest, rank numbers MUST start at 1 and MUST NOT skip a number.
Several candidates MAY share a number: the author ranks them equally and the
reader orders them as they like. Numbers after a tie continue from the next
integer (`1.`, `2.`, `2.`, `3.`), not from the number of candidates so far.

The order of candidate lines is not significant. A formatter orders them: ranked
picks by number, then `+`, then `*`, then `-`, keeping the existing order within
each group.

### 4.8 Section body

After the kind line, a question's body is a *structure block* followed by
*rationale*. The structure block is any sequence of fields, candidate lines,
their continuations, comments, and blank lines. Rationale begins at the first
line that is none of those, and continues to the next heading or end of file.

Rationale is a sequence of paragraphs separated by blank lines. A paragraph is
one or more consecutive non-blank lines joined with single spaces. Paragraphs
use inline markup (section 7).

Only the first line of a paragraph is classified. Later lines of the same
paragraph are prose whatever they begin with, except that a heading line
(section 4.3) and a comment line (section 3.4) are always what they are. A
producer MUST NOT wrap a paragraph so that a continuation line begins with `#`
or `//`; it inserts `\` in front (section 7).

The first line of a paragraph MUST NOT begin with any of these, because the
line would be read as something else:

| Begins with                          | Would be read as      |
|--------------------------------------|-----------------------|
| `#`                                  | a heading             |
| `>`                                  | a description         |
| `+ `, `- `, `* `, or digits then `. `| a candidate line      |
| a lowercase key then `:`             | a field               |
| `//`                                 | a comment             |
| a space                              | a continuation line   |

Once rationale has begun, a paragraph-opening line that looks like a field or
candidate line is an error ("move it above the rationale"), not prose. Begin
the line with `\` to write such a paragraph literally. Note that `**bold**`
and `*emphasis*` at the start of a paragraph are fine: the candidate marker is
`*` followed by a space.

## 5. Data model

The data model is what a parser produces and what every other tool consumes. It
is independent of the text syntax; `oval export --json` is one serialization
of it.

```
Guide
  version:    1
  name:       string
  date:       date
  author:     { name: string, url?: url }
  updated?:   date
  language:   language tag, default "en"
  children:   [Node]

Node = Section | Contest | Measure | Retention
  common to all: id, name, description?: Inline

Section
  region:     bool
  children:   [Node]

Contest
  partisan:   bool
  method:     Single | Multiple { seats: int ≥ 2 } | Ranked { max?: int ≥ 1 }
  writeIns:   bool
  unverified: bool
  party?:     string
  note?:      Inline
  candidates: [Candidate]
  decision:   Undecided | None | Decided
  rationale:  [Inline]

Candidate
  id:          string (unique within the contest)
  name:        string
  party?:      string
  incumbent:   bool
  writeIn:     bool
  designation?: string
  note?:       Inline
  position:    Pick | Rank(n ≥ 1) | Neutral | Avoid

Measure
  unverified: bool
  placedBy?:  string
  yes?:       string
  no?:        string
  threshold?: string
  supporters: [string]
  opponents:  [string]
  note?:      Inline
  vote:       Yes | No | Undecided | None
  rationale:  [Inline]

Retention
  unverified:  bool
  designation?: string
  note?:       Inline
  vote:        Yes | No | Undecided | None
  rationale:   [Inline]

Inline = [Text | Emphasis(Inline) | Strong(Inline) | Code(string)
          | Link(Inline, url) | Ref(Inline, node id)]
```

A contest's `decision` is derived: `Decided` if any candidate has position
`Pick` or `Rank`; otherwise the value of the `vote` field, which is then
REQUIRED. The meaning of the three states is the same for every question:

- `Undecided`: the author has not decided yet; the guide is still being written
  here.
- `None`: the author considered it and makes no recommendation.
- `Decided`: see the positions (contest) or the vote (measure, retention).

An `Avoid` position is independent of the decision: a contest with no picks
and `vote: none` MAY still mark candidates to avoid.

## 6. Semantic rules

A conforming parser MUST report each of these as a diagnostic with the line it
applies to. Rules marked *error* make the file non-conforming; rules marked
*warning* do not. Style and completeness advice (a decided contest with no
rationale, a measure with no `yes` summary) is the linter's job, not the
specification's.

### 6.1 Identifiers

- A section with no explicit `@id` gets a derived id: the name lowercased,
  diacritics removed, every run of characters outside `a-z0-9` replaced by `-`,
  leading and trailing `-` removed. *Warning*: derived ids change when the name
  is edited, so sections SHOULD carry explicit ids.
- Section ids MUST be unique across the file, whether explicit or derived.
  *Error.*
- A candidate with no explicit `@id` gets a derived id by the same rule.
  Candidate ids MUST be unique within their contest. *Error.* Candidate ids are
  scoped to the contest and are not referenceable from rationale.

### 6.2 Structure

- The first heading is level 1; no heading exceeds its parent's level plus one.
  *Error.*
- A question contains no sections. *Error.*
- A container has no fields, candidate lines, or rationale. *Error.*
- Fields and candidate lines appear only after a kind line. *Error.*
- Each field key appears at most once per section; only keys allowed for the
  kind appear. *Error.*
- Each tag appears at most once; only tags allowed for the kind appear;
  `ranked` and `N seats` are not both present; `rank up to N` requires
  `ranked`. *Error.*

### 6.3 Decisions

- A contest with no picked or ranked candidate has `vote: undecided` or
  `vote: none`. *Error.* Absence never means anything.
- A contest with a picked or ranked candidate has no `vote` field. *Error.*
- A measure or retention has a `vote` field. *Error.*
- In a vote-for-one contest at most one candidate is `+`. In an `N seats`
  contest at most N are. *Error.*
- In a ranked contest no candidate is `+`; in any other contest no candidate is
  numbered. *Error.*
- Rank numbers start at 1 with no gaps. *Error.* With `rank up to N`, the
  highest rank is at most N. *Error.*
- A `write-in` candidate appears only in a contest tagged `write-ins`. *Error.*

### 6.4 Values

- `date` and `updated` are valid full-dates. *Error.*
- The author url and link urls are absolute http or https URLs. *Error.*
- A `Ref` link resolves to a section id in this file. *Error.*
- `N` in `N seats` and `rank up to N` is a decimal integer in range. *Error.*

## 7. Inline markup

Inline markup applies to rationale paragraphs, descriptions, notes, and
candidate notes. It is a small fixed subset of Markdown. There is no raw HTML,
no image, no heading, no list, and no hard line break; a line break inside a
paragraph is a space.

| Syntax             | Meaning                                                |
|--------------------|--------------------------------------------------------|
| `*text*`           | Emphasis. Delimiters must be flanking: not followed (opening) or preceded (closing) by whitespace. |
| `**text**`         | Strong.                                                |
| `` `text` ``       | Code. No markup inside. A code span may not span lines.|
| `[text](url)`      | Link to an absolute http or https URL.                 |
| `[text](@id)`      | Reference to a section in this file by id.             |
| `\` + punctuation  | The literal character. Escapable: `\ * _ ` [ ] ( ) # > + - @ /`. |

Underscore is a literal character; `_text_` is not emphasis. A `*` that cannot
pair is literal. Nesting is allowed for emphasis and strong inside links and
each other, to one level.

Consumers that render inline markup to HTML MUST escape text and MUST NOT
render any construct not listed here (section 10).

## 8. Versioning and extension policy

*Outline.* The magic line carries the major version. Within a major, new tags
and fields are added only in ways an older parser reports as unknown; an
`x-` prefix on a tag or key is reserved for private extensions, which parsers
MUST preserve and MUST NOT interpret.

## 9. Media type, file extension, UTI

*Outline.* Extension `.oval`. Media type `text/vnd.oval-vote`, charset always
UTF-8. UTI `vote.oval.guide` conforming to `public.plain-text`. `file(1)`
magic: the string `oval 1` at offset 0.

## 10. Security considerations

*Outline.* Rationale is rendered by viewers: escape all text, allow only the
section 7 constructs, allow only http and https link schemes. Reference links
are intra-document only. No remote inclusion exists in version 1. Parsers
SHOULD bound file size and nesting.

## 11. IANA considerations

*Outline.* The media type registration template, filled in.
