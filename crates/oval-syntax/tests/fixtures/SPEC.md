# Oval Specification

_Working Draft_

**Introduction**

This is the specification for Oval, a plain-text file format for voting guides. It defines the Oval
language, its characters, tokens, grammar, and the meaning of each construct.

For prospective implementers of the specification, it also describes how a conforming lexer, parser,
validator, formatter, and compiler behave.

## 1. Overview

A voting guide tells its readers what is on their ballot and how its author recommends they vote on
each question, with optional reasoning behind each recommendation. Newspapers, advocacy groups,
political clubs, religious organizations, and even individuals often publish guides before every
election, almost always as prose: a web page, a PDF, a thread of posts. Written that way, the
structure of a guide exists only in the author's head and the reader's eye. Publishing the same
guide in more than one place means reformatting it by hand each time. Ballots are harder to model
than they look, because election rules and their regional variations are numerous and inconsistent.
Modeling recommendations is harder still: a recommendation can be nuanced, can rank candidates, can
endorse some and warn against others, or can decline to take a position.

An Oval document represents a single voting guide. It states the election, the ballot or ballots,
the contests and measures on each one, the candidates and parties, and the author's position on each
question. Structure is expressed with keywords and indentation. Reasoning is written as Markdown
under the question it concerns.

```oval
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03

ballot @city "City of Ashford"

  contest @mayor "Mayor" single
    campaign incumbent @bsantos "Beatriz Santos"
      designation "Mayor of Ashford"
    campaign @cfox "Carl Fox"
      designation "Small Business Owner"
    recommend
      support @bsantos
      justification:
        Santos has kept her 2022 promises on housing and transit. Fox has no plan beyond
        cutting a budget he has not read.

  measure @prop-a "Proposition A"
    subtitle "Library parcel tax renewal"
    recommend yes
      justification:
        Renews an existing tax at the same rate. Every branch stays open seven days a week.
```

Oval is designed around four design goals, in no particular order:

- **Readable by hand.** An author with no programming background should be able to write a complete
  guide in a text editor and read it back as plain text. There is nothing to learn beyond a few
  keywords and indentation, and nothing to install to write a guide.
- **Explanations are first class.** A good voting guide is not just a list of selections. It should
  also contain generally useful explanatory information about the election and the author's
  reasoning behind their recommendations. Oval documents don't just model ballots, they model this
  prose as well.
- **Checkable.** A static validator should be able to catch an unfinished or self-contradictory
  guide before it is published, for example: a contest with no recommendation, a reference to a
  candidate who is not in that contest, or ranking recommended in a vote-for-one contest. A voting
  guide is either completely valid or completely invalid.
- **Exportable.** A guide is readable as plain text, but most readers will want it in a richer
  format, such as a web page, a PDF, or a printed card. Tools that produce those kinds of outputs
  will want stricter input than Oval's text format — one with a fixed shape, independent of Oval's
  syntax, with every reference resolved and every count computed, so that rendering requires no
  lookup. A valid guide compiles to one JSON value with a fixed schema, and two guides that mean the
  same thing compile to the same value, so a renderer reads that output and never parses Oval
  itself.

Oval does not describe how a guide is presented, but it does describe how a guide _can_ be presented
using its canonical output. Rendering a guide as a web page, a printed card, a PDF file, or a
comparison table is the job of tools built around the compiled output, and a document carries no
layout, styling, or presentation instructions of its own. Oval also does not model the election
beyond what the guide's author chooses to say about it: there are no results, polling places,
districts, or registration rules, and no attempt to be a source of record for who is running.

**Specification**

This document defines Oval in four parts, each building on the one before it:

- **The language.** The lexical and syntactic grammar, which decides which sequences of characters
  are allowed in an Oval document, and the meaning of each construct. A document that follows the
  grammar is _well-formed_. A conformant parser accepts every well-formed document and reports where
  the grammar was broken in every other one.
- **Validation.** The rules that ensure a voting guide is complete, correct, and consistent. Each
  rule says what it checks and whether breaking it is an `error` or a `warning`. A well-formed
  document that breaks no rule whose severity is `error` is _valid_. A conformant validator reports
  every `error` and every `warning`.
- **Formatting.** The canonical style for Oval documents. A conformant formatter accepts every
  well-formed document and produces an equivalent document in that style. Formatting never changes
  the meaning of a document, never makes a valid document invalid, and never introduces a warning
  into a valid document.
- **Canonical output.** The JSON value a valid document compiles to. It is the guide with every
  reference resolved and with the fields any renderer would otherwise recompute, in a shape fixed by
  a published [JSON Schema](https://json-schema.org). Compiled output should be unaffected by file
  organization, comments, whitespace, and formatting — two guides with the same meaning should
  compile to the same output.

The appendices collect the complete grammar and the list of keywords.

This document does not define the interface of any tool. The names of commands, the wording and
layout of diagnostics, and how an editor presents them belong to the tools. It does not render
Markdown; notes and justifications pass through to the output unchanged, limited to the Markdown
profile of Section 2.2 so that every renderer shows them the same way. And it does not say how
guides are published, distributed, or kept current once written.

Every Oval document begins with a version marker naming the major version of the language it was
written in, so a tool knows how to read it before reading anything else. Within a major version,
each edition of this specification only adds to the language; nothing is removed or changed. A tool
built to a later edition therefore reads every document written to an earlier edition of the same
major version, and a document that uses a construct its tool does not yet know fails at that
construct rather than being misread.

### 1.1. Notation

Grammar productions are written in the following notation. A syntactic production, written
`Name : Rule`, matches tokens; a lexical production, written `Name :: Rule`, matches characters.
Alternatives are listed one per line. `X?` is optional, `X*` is zero or more, and `X+` is one or
more. Literal text is in `backticks`; in a syntactic production it matches a `Word` or `Punctuator`
token with exactly that text. Prose that stands in for a rule is in double quotes. `one of`
introduces alternatives that are each a single token, and `but not` excludes alternatives.
Production names are UpperCamelCase, and on the website every name links to its definition.

The complete grammar is collected in Appendix A.

### 1.2. Identity

| Property                | Value                                                          |
| ----------------------- | -------------------------------------------------------------- |
| Name                    | Oval                                                           |
| Home                    | [`https://oval.vote`](https://oval.vote)                       |
| Source                  | [`https://github.com/oval-vote`](https://github.com/oval-vote) |
| File extension          | `.oval`                                                        |
| Media type              | `text/prs.oval`                                                |
| Uniform type identifier | `vote.oval.document`, conforming to `public.plain-text`        |
| Encoding                | UTF-8                                                          |
| Version marker          | `oval 1`, the first entry of every document                    |
| Canonical output        | JSON, `application/json`                                       |

**File extension.** Oval documents use the extension `.oval`.

**Media type.** The media type is `text/prs.oval`, registered (or to be registered) in the IANA
personal tree under [RFC 6838](https://www.rfc-editor.org/info/rfc6838/), Section 3.3. The `text/`
top-level type is used because an Oval document is readable without interpreting software (RFC 6838
Section 4.2.1). The personal tree is used because the format is currently non-commercial and
experimental; it may move to the vendor tree (`text/vnd.`) if warranted.

The type takes one optional parameter, `charset`, whose only permitted value is `UTF-8`. Oval
documents are always UTF-8; the parameter exists to satisfy the requirement that `text/`
registrations state how the character set is determined, and conveys no information. There is no
structured-syntax suffix: Oval is not JSON, XML, or any other registered structured syntax.

**Uniform type identifier.** On Apple platforms the type is identified as `vote.oval.document`. It
conforms to `public.plain-text`, so any application that opens plain text can open an Oval document,
and its tag specification lists the extension `oval` and the media type `text/prs.oval`. The
identifier is exported by the application that owns the format on a given system and imported by
every other application that reads or writes it; two applications must not both export it.

**Canonical output.** The JSON value a document compiles to is served as `application/json`. It has
no media type of its own; its schema is identified by the `$id` of `oval.schema.json`, which names
the edition the schema belongs to.

**Version marker.** Every document begins with `oval` followed by the major version of the language
it is written in. The marker is the first entry of the document and is defined in Section 2.5.

---

## 2. Language

### 2.1. Source Text

An Oval document is a sequence of Unicode code points encoded as UTF-8. Input that is not
well-formed UTF-8, or that contains U+0000, is rejected as a whole and produces no tokens. A byte
order mark (U+FEFF) at the start of the input is an ignored `ByteOrderMark` token.

SourceCharacter :: "any Unicode code point except U+0000"

A document is divided into _lines_ by line terminators.

LineTerminator ::

- "New Line (U+000A)"
- "Carriage Return (U+000D)" "New Line (U+000A)"
- "Carriage Return (U+000D)"

All three forms are equivalent.

Oval is **layout-sensitive**: the structure of a document is given by the indentation of its lines,
not by brackets or closing keywords.

### 2.2. Lexical Tokens

Token ::

- Word
- String
- Integer
- Date
- Url
- Punctuator
- MarkdownBlock

Tokens are separated by whitespace within a line as necessary. Whitespace is never required between
a `Punctuator` and an adjacent token.

Every byte of the source belongs to exactly one token. Whitespace, comments, line terminators, and a
leading byte order mark are tokens, text that cannot be tokenized becomes an `Error` token, and the
indentation tokens have no text of their own. Concatenating the text of every token therefore
reproduces the source exactly, which is what lets a formatter rewrite a document without losing
anything.

#### Word

Word :: Letter WordPart*

WordPart ::

- Letter
- Digit
- `-`
- `_`

Letter :: /[A-Za-z]/

Digit :: /[0-9]/

A `Word` is a bare run of letters, digits, hyphens, and underscores that begins with a letter. The
lexer accepts every such run; which words may appear where is decided by the syntactic grammar. A
literal in backticks in a syntactic production, such as `ballot`, matches a `Word` whose text is
exactly that, case-sensitively. A word the syntactic grammar names this way is a **keyword**;
Appendix B lists them. A word in a position where the grammar does not expect it is a syntax error,
not a lexical one. Words have no other use: Oval has no free identifiers, and every name an author
chooses is an `Identifier`, a word after `@`.

#### String

String :: `"` StringCharacter* `"`

StringCharacter ::

- SourceCharacter but not `"` or `\` or LineTerminator
- EscapedCharacter

EscapedCharacter :: one of `\"` `\\`

A string is single-line display text. Only two escapes exist: `\"` for a double quote and `\\` for a
backslash. Strings carry no formatting; all multi-line or formatted text is a `MarkdownBlock`.

A line terminator or end of input before the closing quote is an _unterminated string_: the text
from the opening quote to the end of the line becomes an `Error` token and lexing resumes on the
next line. A backslash followed by anything other than `"` or `\\` is an _invalid escape_: the
string is still a `String` token, and the error is reported on the two characters of the escape.

#### Integer

Integer :: Digit+

An unsigned whole number. Leading zeros are permitted, but are insignificant. Integers cannot be
signed.

A word that begins with a Digit but is neither an `Integer` nor a `Date` (such as `12abc`, `3_000`,
`2026-11-5`, or `1.1`) is a _malformed number_ and becomes one `Error` token. Within a word that
begins with a Digit, `.` is a word character, so `1.x` is one malformed number rather than an
`Integer` followed by an unexpected character.

#### Date

Date :: Digit Digit Digit Digit `-` Digit Digit `-` Digit Digit

An ISO 8601 calendar date. The lexer and parser match the shape only. Whether the date exists on the
calendar is handled in the validation rules, so `2026-02-30` would be considered a well-formed
`Date`, but its presence would still be an invalid document.

#### Url

Url :: UrlScheme `://` UrlCharacter+

UrlScheme :: one of `https` `http`

UrlCharacter :: SourceCharacter but not one of "Space (U+0020)", "Tab (U+0009)", LineTerminator

A URL is written bare, without quotes. It extends to the next space, tab, or line terminator, so `#`
and `"` inside a URL are part of it. Only `http` and `https` are permitted. A scheme and `://` with
nothing after them is a _malformed URL_ and becomes one `Error` token. Any other word followed by
`://` is lexed as a `Word`, a `:`, and whatever follows, and fails in the parser.

#### Punctuator

Punctuator :: one of `@` `:` `,`

`@` begins an `Identifier`. `:` introduces a `MarkdownBlock`. `,` separates strings in a list.

#### MarkdownBlock

A `MarkdownBlock` is the body of a Markdown field: the lines following a line that ends in `:`.

MarkdownBlock :: BodyLine+

BodyLine :: "a line indented more than the opening line, or a blank line between two such lines"

The _opening line_ is any line whose last token, ignoring whitespace and comments, is `:`. After its
`Newline`, the lexer looks ahead past any lines that are empty or contain only spaces to the next
line with other content; a `#` is content here, since inside a body it is text. If that line is
indented strictly more than the opening line, the body begins there and extends over every following
line that is either blank or indented strictly more than the opening line, ending after the
terminator of the last such line with content. If the next content line is not indented more, there
is no body and no token.

The token covers the body's raw text, from the first content line's leading spaces through the last
content line's terminator (or end of input). Blank lines before and after the body are not part of
it; they lex as `BlankLine`. Blank lines inside the body are part of it. Body lines do not
participate in layout: they produce no `Newline`, `Indent`, or `Dedent`, and the indentation stack
is unchanged after the body.

The body is **opaque**. Inside it `#` is not a comment, `@`, `"`, and tabs are ordinary characters,
and no other token is recognized. The body is Markdown; the Markdown profile below says which
constructs it may use, and should be enforced during document validation. The lexer does not inspect
the body.

```oval example
note:
  Read *both* arguments.

  # Not a comment
  @ and " are text too.

date 2026-11-03
```

```tokens
Word «note»
Punctuator «:»
Newline «↵»
MarkdownBlock «␣␣Read *both* arguments.↵↵␣␣# Not a comment↵␣␣@ and " are text too.↵»
BlankLine «↵»
Word «date»
Whitespace «␣»
Date «2026-11-03»
Newline «↵»
```

The token's _value_ is derived from its text: the common leading indentation of its non-blank lines
is removed from every line, lines containing only spaces become empty, every line terminator becomes
U+000A, and the final terminator is dropped. Parsers expose the value; the token itself keeps the
raw text.

A `:` followed on its line by anything other than whitespace, a comment, or a line terminator does
not open a body; whether that is valid, and whether the word before the `:` is one that takes a
Markdown body, is the parser's concern. A Markdown field whose opening line has no body is a syntax
error, not a lexical one; omit the field instead.

#### Markdown Profile

The value of a `MarkdownBlock` is Markdown as defined by
[CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/), extended with tables as defined by the
[GitHub Flavored Markdown tables extension](https://github.github.com/gfm/#tables-extension-). Oval
permits a subset of that language, the **Markdown profile**, so that every renderer shows a body the
same way. A body that uses a construct outside the profile is invalid. The profile is not part of
the token: the lexer and parser never inspect a body, validation checks it, and the body reaches the
canonical output unchanged.

A body may contain these block constructs:

- Paragraphs.
- ATX and setext headings. A heading in a body is subordinate to the construct the field belongs to;
  a renderer places it under its own heading for that construct and may change its level.
- Block quotes.
- Bullet lists and ordered lists, nested to any depth.
- Tables.
- Hard line breaks, written as two trailing spaces or a backslash before the line terminator.

And these inline constructs:

- Emphasis and strong emphasis.
- Links: inline links, reference links with their link reference definitions in the same body, and
  autolinks. Every link destination is an absolute `http` or `https` URL, and `http` draws a
  warning.
- Backslash escapes, and entity and numeric character references.

A body may not contain:

- Code, in any form: indented code blocks, fenced code blocks, and code spans.
- Raw HTML, as blocks or inline.
- Images.
- Thematic breaks.

Everything CommonMark does not recognize is rendered as plain text. Strikethrough, task list items,
footnotes, and bare URLs are extensions outside the profile. A body containing them is valid but
shows them as the characters written; write a URL as a link.

Four or more spaces of indentation beyond the body's common minimum begin an indented code block in
CommonMark. A paragraph or list item indented that far is therefore an error; reduce the
indentation. A formatter preserves relative indentation inside a body, so it never introduces or
removes one.

### 2.3. Ignored Tokens

Ignored ::

- Comment
- Whitespace
- BlankLine
- ByteOrderMark

Comment :: `#` CommentCharacter*

CommentCharacter :: SourceCharacter but not LineTerminator

Whitespace :: "Space (U+0020)"+

BlankLine :: "a LineTerminator ending a line with no tokens but Whitespace, Comment, ByteOrderMark"

ByteOrderMark :: "Byte Order Mark (U+FEFF) at the start of the input"

A `#` begins a comment that runs to the end of the line, **except inside a `MarkdownBlock` body or a
`String`**, where it is an ordinary character. A comment may follow tokens on the same line.

A line containing nothing but whitespace, a comment, and a leading byte order mark is a _blank
line_. Its terminator lexes as `BlankLine` rather than `Newline`, so blank lines are ignored by the
parser wherever they appear. Inside a `MarkdownBlock` body blank lines are part of the body and
separate paragraphs.

A byte order mark is a `ByteOrderMark` token only as the first character of the input. U+FEFF
anywhere else is an unexpected character, like any other character outside ASCII.

Horizontal tab (U+0009) is **not** whitespace. A tab anywhere outside a `String`, a `Comment`, or a
`MarkdownBlock` body is a lexical error. This rule exists so that indentation is unambiguous.

### 2.4. Layout Tokens

Oval's block structure is expressed by indentation. The lexer converts indentation into three
synthetic tokens, following the approach of Python.

LayoutToken ::

- Newline
- Indent
- Dedent

Newline :: "the end of a line that contains tokens"

Indent :: "an increase in indentation, at the start of a line"

Dedent :: "a return to an enclosing indentation, at the start of a line or at the end of input"

All three are **zero-length** except `Newline` at a line terminator. `Indent` and `Dedent` are
positioned at the first token of the line that triggers them, after that line's leading
`Whitespace`. Because they have no text, the lossless property of Section 2.2 is preserved.

A line's _indentation_ is the number of U+0020 characters before its first other character. A tab is
an "other character" (and an error). A byte order mark is not: on the first line, indentation is
counted after it. The lexer maintains a stack of indentation levels, initially `[0]`.

- At the end of each line with tokens, emit `Newline` covering the line terminator. A blank line
  emits `BlankLine` instead and does not affect the stack.
- Before the first token of a line with tokens, compare its indentation `n` to the top of the stack:
  - If `n` is greater, push `n` and emit `Indent`.
  - If `n` is less, pop the stack and emit one `Dedent` per level popped until the top is less than
    or equal to `n`. If the top is then not equal to `n`, report an _inconsistent dedent_ over the
    line's leading spaces and treat the line as being at the top's level.
  - If `n` is equal, emit nothing.
- At end of input, if the last line has tokens and no terminator, emit a zero-length `Newline`. Then
  emit `Dedent` for every level above 0.

So a block always closes with the sequence `Newline Dedent`, whether it is followed by another line
or by end of input, and the grammar's productions can end in `Newline` without an end-of-input
special case.

The amount of indentation is not fixed; any increase opens a block. A formatter emits two spaces per
level. A line indented more than its predecessor where the grammar does not permit a block is a
syntax error.

Lines inside a `MarkdownBlock` body do not participate in this process. The entire markdown block is
considered one token.

### 2.5. Document

Document : VersionMarker? GuideField* Declaration* Ballot*

VersionMarker : `oval` Integer Newline

A document is a Guide written at indentation zero. It opens with the version marker, then the
document's fields, then any top-level `Party` and `Candidate` declarations, then the root `Ballot`s.
These four groups appear in that order; within the fields and within the declarations, order is
free. The grammar leaves the ballots optional for the same reason it leaves `guide` optional: a
document with none is invalid, not malformed.

There is no closing token. The document ends at end of input.

**Version marker.** `oval 1` names the major version of the language the document is written in. It
is the first entry of every document; only blank lines and comments may precede it. The grammar
leaves it optional for the same reason it leaves `guide` optional: a parser can then build a tree
for a document that lacks it and say exactly what is missing. A document without a marker, or whose
marker names a major version the implementation does not support, is invalid. The `Integer` is
written without leading zeros; a formatter removes them.

This specification defines major version 1. Within a major version, each edition of this
specification only adds to the language, so an implementation of a later edition accepts every
document written to an earlier one. A construct that an implementation does not know is a lexical or
syntax error at that construct, which is how an older implementation reports a document that needs a
newer edition. A new major version is reserved for a change that removes a construct or alters the
meaning of one.

### 2.6. Fields and Blocks

Most constructs in Oval are a _header line_ followed by an optional _block_ of indented lines. The
block contains _fields_ and _children_.

Block : Indent BlockEntry+ Dedent

BlockEntry :: "a field or a child of the construct, as its section lists"

A **field** is a keyword followed by its value and a `Newline`, or, for Markdown, a keyword and a
colon on one line followed by a `MarkdownBlock`:

MarkdownBody : `:` Newline MarkdownBlock

A value is a `String`, `Integer`, `Date`, `Url`, `Identifier`, or string list:

StringList :

- `none`
- Strings

Strings :

- String
- Strings `,` String

Each construct's section lists the fields it accepts and the value each takes.

Within one block, **each field keyword may appear at most once**, and **fields may appear in any
order**. A repeated field is a syntax error. **Children** (campaigns within a contest, items within
a ballot) are ordered, and their order is significant. Fields and children may be interleaved in the
source, though the canonical formatting style described later in this document contains a preferred
order.

A `StringList` written as `none` denotes an empty list. This is distinct from omitting the field.

A **flag** is a keyword on a header line whose presence means `true`:

Flags : Flag*

Flag : one of `incumbent` `withdrawn`

There is no `false` spelling; omit the flag.

A **requirement** is a keyword written before the keyword of a `Ballot`, `Section`, `Contest`, or
`Measure`, on the same line:

Requirement : one of `required` `optional`

A question (a contest or a measure) is **required** when the ballot requires the voter to answer it,
and **optional** when the voter may leave it blank. Every question has exactly one of these values,
its _effective requirement_, found by looking outward from the question to the nearest construct
that has a requirement written on it: the question itself, then its enclosing section, if any, then
its enclosing ballots from the innermost out to the root ballot. When none of them has one, the
question is optional.

```oval example
required ballot @city "City of Ashford"
  section "City Offices"
    contest @mayor "Mayor" single
      ...
    optional contest @auditor "City Auditor" single
      ...
  measure @prop-a "Proposition A"
    ...
```

Here `@mayor` and `@prop-a` are required, inherited from the ballot, and `@auditor` is optional
because its own requirement is nearer. A requirement on a ballot or a section has no meaning of its
own; it only supplies the value its questions inherit. A question's effective requirement decides
whether it may be recommended with `abstain`.

An **extensibility** is a keyword written before the keyword of a `Ballot`, `Section`, or `Contest`,
after its requirement if it has one:

Extensibility : one of `extensible` `exhaustive`

A contest is **extensible** when the voter may write in a choice that is not printed on the ballot,
and **exhaustive** when the printed choices are the only ones. Which it is, its _effective
extensibility_, is found the same way as its requirement: from the nearest extensibility written on
the contest, its section, or its enclosing ballots, and exhaustive when there is none. Two kinds of
question are always exhaustive, whatever they inherit: a `retention` contest, which asks only yes or
no, and a measure, which takes no extensibility of its own. Only an extensible contest's
recommendation may contain a `write-in`.

```oval example
required extensible ballot @city "City of Ashford"
  contest @mayor "Mayor" single
    ...
  exhaustive contest @auditor "City Auditor" single
    ...
  contest @judge-ueda "Retain Judge Ueda?" retention
    ...
```

Here `@mayor` is extensible, inherited from the ballot. `@auditor` is exhaustive because its own
extensibility is nearer, and `@judge-ueda` is exhaustive because it is a retention contest. All
three are required.

### 2.7. Declarations and References

Entities (Party, Candidate, Ballot, Contest, Measure) are _declared_ exactly once each and may be
_referred to_ any number of times.

Identifier : `@` Word

Reference : Identifier

An identifier names an entity. It is the punctuator `@` followed by a `Word`, written without
whitespace between them; a formatter removes any. Identifiers are case-sensitive, and two
identifiers are the same if their words are identical. Because every identifier begins with `@`, no
word is reserved: `@ballot` is an ordinary identifier.

A declaration is an `Identifier` **followed immediately by a `String`**, after the entity's keyword;
the string is the entity's name or title. A reference is a bare `Identifier`. This is the single
rule for every entity kind:

```
candidate @jlee "Jordan Lee"     — declares @jlee
campaign @jlee                   — refers to @jlee
campaign @kobi "Kwame Obi"       — declares @kobi here, inline
party @dem "Democratic"          — declares @dem
party @dem                       — refers to @dem
```

Some entities may be declared **inline** at their first use instead of at the top of the document.
Where a reference is permitted, the grammar says whether a declaration is also permitted in that
position. An inline declaration is the same declaration as a top-level one; it is simply written
elsewhere. Validation requires every reference to name a declaration, and every identifier to be
declared at most once, wherever that is.

Identifiers share one namespace across all entity kinds.

### 2.8. Guide

The Guide is the document. Its fields are written at indentation zero.

GuideField :

- `guide` String Newline
- `election` String Newline
- `date` Date Newline
- `note` MarkdownBody

```oval example
oval 1

guide "Franklin Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03

note:
  Written by the Franklin Civic Alliance.
```

`guide`, `election`, and `date` are required. `note` is optional. The version marker that precedes
them belongs to the document, not to the guide.

Declaration :

- Party
- Candidate

Top-level declarations follow the guide fields. They are for parties and candidates referenced from
more than one place; anything referenced once may be declared inline instead.

### 2.9. Party

Party : `party` Identifier String Newline PartyBlock?

PartyBlock : Indent PartyField+ Dedent

PartyField :

- `abbreviation` String Newline
- `website` Url Newline
- `note` MarkdownBody

```oval example
party @dem "Democratic"
  abbreviation "D"
```

A party may be declared at the top level, inline in a candidate's `party` field, or inline as a
member of a party campaign.

### 2.10. Candidate

Candidate : `candidate` Identifier String Newline CandidateBlock?

CandidateBlock : Indent CandidateField+ Dedent

CandidateField :

- `designation` String Newline
- PartyRef
- `note` MarkdownBody

PartyRef :

- `party` Identifier Newline
- `party` Identifier String Newline PartyBlock?

```oval example
candidate @nreyes "Nadia Reyes"
  designation "United States Senator"
  party @dem
  note:
    Appointed in January 2025 to fill a vacancy.
```

`PartyRef`'s second form is an **inline party declaration**: `party @green "Green"` declares
`@green` right here and makes it this candidate's party. It may be followed by a `PartyBlock`
indented under it.

A candidate may be declared at the top level, as above, or inline in a campaign. Declare at the top
level when the person appears in more than one contest.

### 2.11. Ballot

Ballot : Requirement? Extensibility? `ballot` Identifier String Newline BallotBlock

BallotBlock : Indent BallotEntry+ Dedent

BallotEntry :

- BallotField
- BallotItem

BallotField :

- `eligibility` String Newline
- `note` MarkdownBody

BallotItem :

- Section
- Contest
- Measure
- Ballot

```oval example
ballot @franklin "State of Franklin"
  section "Federal Offices"
    ...
  ballot @ashford "City of Ashford"
    eligibility "Registered voters of the City of Ashford"
    ...
```

A ballot is a node in the guide's tree. Its items are in ballot order. A `Ballot` item is a
**sub-ballot**: a narrower electorate whose voters also see everything in the enclosing ballot.
Sub-ballots nest without limit. Document-level ballots are **root ballots**; a valid document has at
least one. A `Requirement` or `Extensibility` before `ballot` is inherited by every question in the
ballot and its sub-ballots that does not have a nearer one.

A ballot with no block is a syntax error, and a ballot whose block has no item is invalid.

### 2.12. Section

Section : Requirement? Extensibility? `section` String Newline SectionBlock

SectionBlock : Indent SectionEntry+ Dedent

SectionEntry :

- SectionField
- SectionItem

SectionField : `note` MarkdownBody

SectionItem :

- Contest
- Measure

A section is a titled group of questions within a ballot, matching a heading on the printed ballot.
It has no identifier and no eligibility meaning. Sections do not nest and do not contain
sub-ballots. A `Requirement` or `Extensibility` before `section` is inherited by every question in
the section that does not have its own.

```oval example
required section "Statewide Offices"
  contest @governor "Governor" single
    ...
  optional contest @controller "Controller" single
    ...
```

### 2.13. Contest

Contest : Requirement? Extensibility? `contest` Identifier String ContestKind Newline ContestBlock

ContestKind :

- `single` Seats?
- `multiple` Integer MultipleModifier*
- `ranked` RankedModifier*
- `retention`

Seats : `seats` Integer

Minimum : `min` Integer

RankedMax : `max` Integer

MultipleModifier :

- Seats
- Minimum

RankedModifier :

- Seats
- RankedMax
- Minimum
- `all`

ContestBlock : Indent ContestEntry+ Dedent

ContestEntry :

- ContestField
- Campaign
- ContestRecommend

ContestField :

- `subtitle` String Newline
- `tabulation` String Newline
- `note` MarkdownBody

```oval example
contest @mayor "Mayor" ranked max 5
  subtitle "Rank up to five candidates"
  tabulation "Instant runoff; majority of continuing ballots wins"

  campaign incumbent @bsantos "Beatriz Santos"
    designation "Mayor of Ashford"
  campaign @aokafor "Ada Okafor"
    designation "Educator / Nonprofit Director"

  recommend
    support @aokafor
    support @bsantos
    justification:
      ...
```

The contest kind is written on the header line after the title, followed by the modifiers the kind
accepts:

| Kind           | Meaning                                                                      | Modifiers                    |
| -------------- | ---------------------------------------------------------------------------- | ---------------------------- |
| `single`       | Vote for one.                                                                | `seats`                      |
| `multiple` _n_ | Vote for up to _n_, where _n_ is at least 2.                                 | `seats`, `min`               |
| `ranked`       | Rank choices in order of preference.                                         | `seats`, `max`, `min`, `all` |
| `retention`    | Yes or no on one candidate: a retention, or a sole candidate's confirmation. | none                         |

| Modifier    | Meaning                                                                 | Default                                  |
| ----------- | ----------------------------------------------------------------------- | ---------------------------------------- |
| `seats` _n_ | How many candidates are elected, at least 1.                            | 1, or the number of votes for `multiple` |
| `max` _n_   | How many choices a voter may rank, at least 1.                          | unlimited                                |
| `min` _n_   | How many choices a ballot that marks the contest must mark, at least 1. | 1                                        |
| `all`       | A ballot that marks the contest must rank every printed choice.         | not required                             |

Each modifier appears at most once, in any order. `all` excludes `max` and `min`, and `min` may not
exceed `max` or, for `multiple`, the number of votes.

```oval example
contest @legco "Legislative Council, Hong Kong Island" single seats 2
contest @atlarge "City Council At-Large" multiple 5 seats 7
contest @council "City Council" ranked seats 3 max 6
contest @house "House of Representatives, Division of Wills" ranked all
```

The first is the single non-transferable vote: one vote, two winners. The second is limited voting,
as in Philadelphia's at-large council race: five votes, seven winners. The third is the single
transferable vote with three winners and at most six rankings, as in Portland. The fourth requires
every box to be numbered, as for the Australian House of Representatives.

`min` counts the marks a ballot needs when it marks the contest at all. Whether the contest may be
left blank is decided by its requirement, not by `min`, so the two combine freely:

```oval example
contest @board "School Board" multiple 3 seats 4 min 2
required contest @commission "County Commission" multiple 3 min 3
```

The first elects four and accepts two or three marks, or none. The second must be answered, and only
with all three votes: a full-slate rule.

A `retention` contest asks for a yes or no on its one candidate. When the campaign is `incumbent`,
it is a retention vote, as for a judge; otherwise it is a confirmation, as when a sole candidate for
mayor in Poland must still win a yes vote.

A contest's block contains its `Campaign`s in ballot order and its `ContestRecommend`s. The grammar
accepts any number of each; validation requires at least one campaign except in a retention contest
and exactly one recommendation, so that a half-written contest still parses and the validator can
name what is missing. Campaigns and the recommendation may be interleaved with fields in the source;
a formatter orders them fields, campaigns, recommendation.

A `Requirement` before `contest` sets the contest's effective requirement directly, and an
`Extensibility` sets its effective extensibility, each overriding any inherited from its section or
ballots. A retention contest is always exhaustive, and writing `extensible` before one is an error.

```oval example
extensible contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support write-in "Jane Doe"
```

### 2.14. Campaign

Campaign :

- `campaign` Flags Member Newline CampaignBlock?
- `campaign` Flags Newline CampaignBlock
- `campaign` Flags `party` PartyMember Newline PartyCampaignBlock?
- `campaign` Flags `party` Newline PartyCampaignBlock

CampaignBlock : Indent CampaignEntry+ Dedent

CampaignEntry :

- Member Newline MemberBlock?
- CampaignField

Member : Identifier String? MemberRole?

MemberRole : `as` String

MemberBlock : Indent CandidateField+ Dedent

CampaignField :

- `website` Url Newline
- `note` MarkdownBody
- `designation` String Newline
- PartyRef

PartyMember : Identifier String?

PartyCampaignBlock : Indent PartyCampaignEntry+ Dedent

PartyCampaignEntry :

- PartyMember Newline PartyBlock?
- PartyCampaignField

PartyCampaignField :

- `website` Url Newline
- `note` MarkdownBody

A campaign is one or more candidates, or one or more parties, running together for the contest. It
has no identifier; it is identified by its members. A campaign's members are all candidates or all
parties: a **candidate campaign** is written `campaign`, and a **party campaign** is written
`campaign party`.

A `Member` is a candidate reference (`@id`) or an inline candidate declaration (`@id "Name"`),
optionally followed by `as String`, the member's role on the ticket ("Governor", "Vice President").
A `MemberBlock` holding candidate fields may follow a member **only when the member is an inline
declaration**; a reference has nothing to declare.

The two forms of `Campaign`:

**One-line form.** The single member is on the header line.

```oval example
campaign incumbent @bsantos "Beatriz Santos"
  designation "Mayor of Ashford"
  website https://santosformayor.example
  note:
    Two-term incumbent.
```

In this form the block contains campaign fields _and_, when the member is an inline declaration, the
candidate's `designation` and `party`. The block's `note` is the **campaign's** note. A candidate
note for a one-line campaign requires the block form below. This is a deliberate simplification: for
a single-candidate campaign the reader sees both notes together regardless.

**Block form.** Members are lines in the block, each optionally followed by its own `MemberBlock` of
candidate fields.

```oval example
campaign incumbent
  @jlee "Jordan Lee" as "Governor"
    designation "Governor of Franklin"
    party @dem
    note:
      First-term governor.
  @mpatel "Maya Patel" as "Lieutenant Governor"
    designation "State Senator"
    party @dem
  website https://leepatel2026.example
  note:
    The ticket's note.
```

Member order is significant: the first member is the **head** of the campaign, the head of the
ticket for candidates and the lead party of a coalition. In block form, `designation` and `PartyRef`
are not permitted at the campaign level; they belong under a member.

**Party campaigns.** A party campaign offers the voter a party rather than a candidate: a party
list, a straight-party box, or a coalition of parties that appears on the ballot as one choice. Its
members are parties, written as a `PartyMember`: a party reference (`@id`) or an inline party
declaration (`@id "Name"`). Party members take no role. Like a candidate campaign, it has a one-line
form for a single party and a block form for several.

```oval example
contest @assembly "National Assembly" single
  campaign party @dem
  campaign party
    @green "Green Party"
      abbreviation "G"
    @labour
    website https://redgreen.example
  campaign @kobi "Kwame Obi"
  recommend
    support @green
```

Here the voter may choose the Democratic Party, the coalition of the Green and Labour parties, or
the independent Kwame Obi. In the one-line form the block holds only the campaign's `website` and
`note`; a party with fields of its own is declared in the block form, where an inline member may be
followed by a `PartyBlock`, or at the top level. A `PartyBlock` may follow a member only when the
member is an inline declaration.

A candidate's `party` field describes the candidate and does not make the party a member of any
campaign. Lists of a party's candidates are not modeled; describe them in the campaign's `note`.

**Flags.** Flags apply to both kinds of campaign. `incumbent` means this campaign currently occupies
the office being contested; `withdrawn` means the campaign has ended but remains on the ballot. Both
default to false. They may appear in either order and each at most once.

### 2.15. Contest Recommendation

ContestRecommend :

- `recommend` Newline StanceBlock
- `recommend` NoStance Newline RecommendFieldBlock?

NoStance : one of `no-position` `abstain`

StanceBlock : Indent RecommendField* Stance RecommendEntry* Dedent

RecommendEntry :

- Stance
- RecommendField

Stance :

- Support
- Oppose

Support : `support` Choice+ Newline

Oppose : `oppose` Choice+ Newline

Choice :

- Reference
- WriteIn

WriteIn : `write-in` String?

RecommendField :

- `justification` MarkdownBody
- `note` MarkdownBody

RecommendFieldBlock : Indent RecommendField+ Dedent

```oval example
recommend
  support @aokafor
  support @bsantos @dwright
  oppose @cfox
  justification:
    Okafor is our first choice. Santos and Wright are acceptable in either order.
  note:
    Do not rank Fox.
```

A recommendation is a position followed by optional commentary. The position is written either on
the `recommend` line, as `no-position` or `abstain`, or as the `support` and `oppose` lines of the
block, never both. Without a `NoStance`, the block is a `StanceBlock`: it is required and contains
at least one `support` or `oppose` line, anywhere among its `justification` and `note`. With one,
the block is a `RecommendFieldBlock`: it is optional and contains only `justification` and `note`.
Measures use the same `RecommendFieldBlock`.

Each `support` line is one **tier**. Several `support` lines are several tiers, in order from most
to least preferred. Choices within one line are interchangeable. `oppose` lists choices the author
advises against; it is unordered and may appear at most once.

A **choice** is a reference or a write-in. Campaigns have no identifiers. Every reference in
`support` or `oppose` names a campaign _by one of its members_: a candidate identifier for a
candidate campaign, or a party identifier for a party campaign. It resolves to the one campaign,
among the enclosing contest's campaigns, whose members include that candidate or party. A
candidate's `party` field is not membership, so naming a candidate never resolves to a party
campaign. A candidate or party may not be a member of more than one campaign within the same
contest. Any member may be named; a formatter rewrites the reference to name the campaign's head.

A **write-in** is a choice the voter writes on the ballot by hand. It is not a campaign, since it is
not printed, and is valid only in an extensible contest. `write-in` followed by a string names the
person to write in. A bare `write-in` names no one: in `support` it means writing in anyone the
reader chooses, and in `oppose` it means writing in no one. A write-in takes its place in the tiers
like any other choice.

```oval example
recommend
  support write-in "Jane Doe"
  support @bsantos write-in "Hiro Eriksen"
  oppose @cfox write-in
  justification:
    Write in Doe. If you would rather not, Santos and Eriksen are both acceptable.
```

Here the reader is advised to write in Jane Doe, or else to vote for Santos or write in Eriksen, and
in no case to vote for Fox or write in anyone else.

`recommend no-position` is an explicit statement that the author takes no position. It may carry
`justification` and `note` but no `support` or `oppose`.

`recommend abstain` is an explicit recommendation to leave the question blank. It differs from
`no-position`: `no-position` says the author offers no advice, while `abstain` advises the reader
not to vote on the question at all. Like `no-position`, it may carry `justification` and `note` but
no `support` or `oppose`. A required question cannot be left blank, so `abstain` is valid only on an
optional one.

```oval example
contest @auditor "City Auditor" single
  campaign @hkim "Hana Kim"
  recommend abstain
    justification:
      The only candidate is unopposed and has not answered our questionnaire.
```

### 2.16. Measure

Measure : Requirement? `measure` Identifier String Newline MeasureBlock

MeasureBlock : Indent MeasureEntry+ Dedent

MeasureEntry :

- MeasureField
- MeasureRecommend

MeasureField :

- `subtitle` String Newline
- `description` MarkdownBody
- `if_yes` MarkdownBody
- `if_no` MarkdownBody
- `placement` String Newline
- `tabulation` String Newline
- `fiscal_impact` String Newline
- `supporters` StringList Newline
- `opponents` StringList Newline
- `note` MarkdownBody

```oval example
measure @prop1 "Proposition 1"
  subtitle "Affordable Housing Bond"
  description:
    Authorizes $2 billion in general obligation bonds ...
  if_yes:
    The state could sell $2 billion in bonds.
  if_no:
    The state could not sell these bonds.
  placement "Legislative referral"
  tabulation "Simple majority"
  fiscal_impact "About $140 million a year for 30 years"
  supporters "Franklin Housing Coalition", "Franklin Labor Federation"
  opponents none

  recommend yes
    justification:
      ...
```

`description`, `if_yes`, and `if_no` are official ballot text copied verbatim. `supporters` and
`opponents` have three states: **absent** (not researched), **`none`** (researched; none known), and
a list. A measure contains exactly one `MeasureRecommend`. A `Requirement` before `measure` sets the
measure's effective requirement directly, overriding any inherited from its section or ballots. A
measure takes no `Extensibility` and ignores one written on its section or ballots.

### 2.17. Measure Recommendation

MeasureRecommend : `recommend` MeasurePosition Newline RecommendFieldBlock?

MeasurePosition : one of `yes` `no` `no-position` `abstain`

```oval example
recommend no-position
  justification:
    Two board members recused and the rest were split.
```

A measure has no campaigns, so its position is always on the `recommend` line and its block holds
only `justification` and `note`. `yes` and `no` recommend a vote. `no-position` says the author
offers no advice. `abstain` advises the reader to leave the measure blank, and is valid only on an
optional measure, as for a contest.

---

## 3. Validation

Validating a document is three stages run in order, each on the output of the one before:

1. **Lexing** turns the source text into the tokens. Text that cannot be tokenized becomes an
   `Error` token and a lexical error. Lexing covers every byte of the input, so the token stream is
   complete even when it contains errors. An exhaustive list of lexical errors is described below.
2. **Parsing** matches the token stream against the syntactic grammar and builds a tree. A token the
   grammar does not expect is a syntax error. A document that lexes and parses without error is
   _well-formed_. An exhaustive list of syntax errors is described below.
3. **Validation** checks the tree against the list of validation rules. A well-formed document that
   produces no error is _valid_. Warnings do not affect validity, but a compliant validator should
   still report them.

Every error at every stage has an identifier and a range in the source. Identifiers are stable
across editions; tools should report errors by identifier. A validator reports every lexical error,
every syntax error, and every rule the document breaks, in source order, so that an author can see
them all in one pass.

### 3.1. Lexical Errors

Where the text cannot be tokenized at all it becomes an `Error` token spanned exactly by one error;
where a token is recognizable but flawed, the token keeps its kind and the error points at the flaw
within it.

After an error the lexer continues, using the recovery given for each error below, so that every
error in a document is reported in one pass and so that the token stream still covers every byte.
The recoveries are normative: two implementations given the same broken document produce the same
tokens and the same errors.

A tool that only needs to accept or reject a document may stop at the first error instead. It then
reports a subset of the errors this section specifies and is not lexer- or parser-conformant, since
those levels exist for tools that must make sense of every byte of a broken file; the documents it
accepts are exactly the documents a conformant implementation accepts.

Note: Each error below is followed by an example that lexes without error and a counter-example that
produces the error, each with its token sequence: every token in order, with its kind and its text.
Each counter-example is also followed by its errors: every error in source order, with its
identifier and the text it spans. A space is shown as ␣, a tab as ⇥, and a line terminator as ↵.
Zero-length tokens have no text.

#### Illegal Tab

Occurs when a tab (U+0009) appears outside a `String`, a `Comment`, or a `MarkdownBlock` body; a run
of consecutive tabs is one error. The error is reported as `illegal-tab` and spans the run of tabs,
which become one `Error` token. To recover, swallow the tabs and continue after them.

```oval example
subtitle "Vote for one"
```

```tokens
Word «subtitle»
Whitespace «␣»
String «"Vote for one"»
Newline «↵»
```

```oval counter-example
subtitle	"Vote for one"
```

```tokens
Word «subtitle»
Error «⇥»
String «"Vote for one"»
Newline «↵»
```

```errors
illegal-tab «⇥»
```

#### Unexpected Character

Occurs when, at a position where a token or an ignored token begins, the character cannot begin one:
anything other than a letter, a digit, `@`, `"`, `:`, `,`, `#`, a space, a line terminator, or a
tab. This includes `-`, `_`, `\`, and every character outside ASCII, since `Letter` is ASCII only;
such characters are permitted inside strings, comments, and bodies. The error is reported as
`unexpected-character` and spans that one character, which becomes an `Error` token. To recover,
swallow the character and continue.

```oval example
candidate @dana-lombardi "Dana Lombardi"
```

```tokens
Word «candidate»
Whitespace «␣»
Punctuator «@»
Word «dana-lombardi»
Whitespace «␣»
String «"Dana Lombardi"»
Newline «↵»
```

```oval counter-example
candidate @-lombardi "Dana Lombardi"
```

```tokens
Word «candidate»
Whitespace «␣»
Punctuator «@»
Error «-»
Word «lombardi»
Whitespace «␣»
String «"Dana Lombardi"»
Newline «↵»
```

```errors
unexpected-character «-»
```

#### Unterminated String

Occurs when a line terminator or end of input appears between an opening `"` and its closing `"`. A
backslash immediately before the line terminator is part of this error, not an invalid escape. The
error is reported as `unterminated-string` and spans the opening quote to the end of the line, which
becomes one `Error` token. To recover, swallow the rest of the line up to, but not including, the
line terminator, then continue so that the terminator lexes normally.

```oval example
election "November 3, 2026 General Election"
date 2026-11-03
```

```tokens
Word «election»
Whitespace «␣»
String «"November 3, 2026 General Election"»
Newline «↵»
Word «date»
Whitespace «␣»
Date «2026-11-03»
Newline «↵»
```

```oval counter-example
election "November 3, 2026 General Election
date 2026-11-03
```

```tokens
Word «election»
Whitespace «␣»
Error «"November 3, 2026 General Election»
Newline «↵»
Word «date»
Whitespace «␣»
Date «2026-11-03»
Newline «↵»
```

```errors
unterminated-string «"November 3, 2026 General Election»
```

#### Invalid Escape

Occurs when, inside a string, `\` is followed by any character other than `"` or `\`. The error is
reported as `invalid-escape` and spans the backslash and the character after it; the string remains
a `String` token. To recover, keep both characters in the string and continue lexing the string
after them.

```oval example
subtitle "The \"People's\" Measure"
```

```tokens
Word «subtitle»
Whitespace «␣»
String «"The \"People's\" Measure"»
Newline «↵»
```

```oval counter-example
subtitle "Measure \#3"
```

```tokens
Word «subtitle»
Whitespace «␣»
String «"Measure \#3"»
Newline «↵»
```

```errors
invalid-escape «\#»
```

The token sequence is the same in both cases; the error is reported on the two characters `\#`
inside the `String`.

An invalid escape in an unterminated string is not reported. The string becomes an `Error` token,
and its unterminated-string error is the only error for that text.

```oval counter-example
subtitle "Measure \#3
```

```tokens
Word «subtitle»
Whitespace «␣»
Error «"Measure \#3»
Newline «↵»
```

```errors
unterminated-string «"Measure \#3»
```

#### Malformed Number

Occurs when a run of letters, digits, `-`, `_`, and `.` begins with a digit and is neither an
`Integer` nor a `Date`. The error is reported as `malformed-number` and spans the run, which becomes
one `Error` token. To recover, swallow the run and continue after it.

```oval example
date 2026-11-03
```

```tokens
Word «date»
Whitespace «␣»
Date «2026-11-03»
Newline «↵»
```

```oval counter-example
date 2026-11-3
```

```tokens
Word «date»
Whitespace «␣»
Error «2026-11-3»
Newline «↵»
```

```errors
malformed-number «2026-11-3»
```

#### Malformed URL

Occurs when `http://` or `https://` is followed immediately by a space, a tab, a line terminator, or
end of input. The error is reported as `malformed-url` and spans the scheme and `://`, which become
one `Error` token. To recover, swallow them and continue with the character after `://`.

```oval example
website https://santosformayor.example
```

```tokens
Word «website»
Whitespace «␣»
Url «https://santosformayor.example»
Newline «↵»
```

```oval counter-example
website https://
```

```tokens
Word «website»
Whitespace «␣»
Error «https://»
Newline «↵»
```

```errors
malformed-url «https://»
```

#### Inconsistent Dedent

Occurs when a line with tokens is indented less than the enclosing block and equal to no enclosing
level. The error is reported as `inconsistent-dedent` and spans the line's leading spaces, which
remain a `Whitespace` token. To recover, emit `Dedent` tokens down to the nearest enclosing level,
treat the line as at that level, and continue.

```oval example
ballot @city "City of Ashford"
  section "Offices"
    contest @mayor "Mayor" single
  measure @prop-a "Proposition A"
```

The last line returns to the section's level, so the document lexes as:

```tokens
Word «ballot»
Whitespace «␣»
Punctuator «@»
Word «city»
Whitespace «␣»
String «"City of Ashford"»
Newline «↵»
Whitespace «␣␣»
Indent
Word «section»
Whitespace «␣»
String «"Offices"»
Newline «↵»
Whitespace «␣␣␣␣»
Indent
Word «contest»
Whitespace «␣»
Punctuator «@»
Word «mayor»
Whitespace «␣»
String «"Mayor"»
Whitespace «␣»
Word «single»
Newline «↵»
Whitespace «␣␣»
Dedent
Word «measure»
Whitespace «␣»
Punctuator «@»
Word «prop-a»
Whitespace «␣»
String «"Proposition A"»
Newline «↵»
Dedent
```

```oval counter-example
ballot @city "City of Ashford"
  section "Offices"
    contest @mayor "Mayor" single
   measure @prop-a "Proposition A"
```

Three spaces match no enclosing level. The nearest enclosing level is the section's, so the line is
treated as if indented two spaces and the document lexes as:

```tokens
Word «ballot»
Whitespace «␣»
Punctuator «@»
Word «city»
Whitespace «␣»
String «"City of Ashford"»
Newline «↵»
Whitespace «␣␣»
Indent
Word «section»
Whitespace «␣»
String «"Offices"»
Newline «↵»
Whitespace «␣␣␣␣»
Indent
Word «contest»
Whitespace «␣»
Punctuator «@»
Word «mayor»
Whitespace «␣»
String «"Mayor"»
Whitespace «␣»
Word «single»
Newline «↵»
Whitespace «␣␣␣»
Dedent
Word «measure»
Whitespace «␣»
Punctuator «@»
Word «prop-a»
Whitespace «␣»
String «"Proposition A"»
Newline «↵»
Dedent
```

```errors
inconsistent-dedent «␣␣␣»
```

### 3.2. Syntax Errors

The parser reads the token stream one _entry_ at a time. An entry is a line together with the block,
if any, indented beneath it. The grammar says which entries each block accepts and which tokens each
line may carry, and the parser reports a syntax error at the first token it cannot accept. A
document with a syntax error is not well-formed.

Every syntax error has a recovery, so that two implementations keep and discard the same text and
report the same errors for the same broken document, and so that an editor can keep working on a
half-written file. This specification does not define the shape of the tree; a recovery fixes which
text each construct keeps and which text is discarded or set aside. A tool that only needs to accept
or reject a document may stop at the first error on the terms Section 3.1 gives. There are four
recoveries, named here and referred to by each error below:

- **Skip the line.** The tokens from the unexpected one to the end of the line are discarded. The
  construct keeps whatever was parsed before them, and parsing continues on the next line. A
  `MarkdownBlock` after the line belongs to it, since the line's `:` opened the body, so it is
  discarded with the line and is not a separate error.
- **Skip the entry.** The entire entry is discarded, including its block or body. Parsing continues
  with the next entry at the same level.
- **Assume the missing token.** Nothing is discarded. Parsing continues as if the token were
  present; the construct is kept with nothing in that position, and validation treats it as absent.
- **Set the entry aside.** The entry is parsed in full, so that errors inside it are still reported,
  but it contributes nothing to the construct that contains it.

Discarded and set-aside text remains in the tree inside an _error node_, so the tree still covers
every byte of the source and a formatter can refuse the document rather than guess at it.

The errors below are exhaustive, by position. A token the grammar cannot accept at the start of an
entry is an unexpected entry, or an unexpected block if it is an `Indent`; one in the middle of a
line is an unexpected token; and a line or entry that ends where the grammar requires more is one of
the missing-token errors. Two recommendation errors refine these with their own identifiers:
`empty-recommendation` for a `StanceBlock` that ends without a stance, and `unexpected-stance` for a
stance in a `RecommendFieldBlock`. The remaining errors are entries that fit the grammar but not the
document's structure. A parser never needs a fallback error.

The parser skips `Error` tokens from the lexer as it skips comments and whitespace, so a lexical
error is the only diagnostic for the text it covers. On a line that contains an `Error` token, no
missing-token error is reported, since what is missing may be the text the lexer rejected. A line
whose only token the parser does not skip is its terminator is treated as blank.

Error ranges are measured over the tokens the parser does not skip. The _end of a line_ is the end
of the last token on it, other than its terminator, that the parser does not skip, so whitespace, a
comment, or an `Error` token after that point is not part of the line. An error that spans part of a
line, a line, an entry, or a block begins at the first token in it that the parser does not skip and
ends at the end of its last line, and a recovery that discards or sets aside that text leaves what
comes before and after it in place. An error that is zero-length at the end of a line is at that
position.

#### Unexpected Token

Occurs when a line carries a token the grammar does not accept at that point and the line has not
ended: a value of the wrong kind, a token after a complete entry, a second value where only one is
allowed, or text on the same line after a Markdown field's `:`. The error is reported as
`unexpected-token` and spans from that token to the end of the line. To recover, skip the line. The
diagnostic names what was expected. A contest-kind modifier the kind does not accept, such as `max`
after `single` or anything after `retention`, is an unexpected token.

```oval example
date 2026-11-03
```

```oval counter-example
date "November 3, 2026"
```

The `date` field is kept with no value; the string is discarded.

```oval counter-example
subtitle "Vote for one" "Pick one"  # which wording?
```

The error spans only the second string. The comment after it is not part of the line's range, so it
is neither reported nor discarded.

```oval counter-example
guide "Franklin Voter Guide":
  Written by the Franklin Civic Alliance.
```

The `:` is the unexpected token. The `guide` field keeps its name, and the body after the line is
discarded with it rather than reported again.

#### Missing Identifier

Occurs when an identifier is required and the line ends, or a `String` follows, where it should
begin: after `ballot`, `contest`, `measure`, `party`, or `candidate`; after `party` used as a field;
and after `support` or `oppose`, which require at least one choice, a reference or a `write-in`. It
also occurs when `@` is not followed by a `Word`. Any other token in that position is an unexpected
token. The error is reported as `missing-identifier` and is zero-length at the position where the
identifier would begin. To recover, assume the missing token and continue with the rest of the line;
after `support` or `oppose`, a `String` in the identifier's place is skipped with it, since it is
not a second error. A name written there is usually a write-in missing its keyword: write
`write-in "Name"`.

```oval example
party @dem "Democratic"
```

```oval counter-example
party "Democratic"
```

The party is kept with its name and no identifier.

#### Missing Name

Occurs when a declaration's identifier is followed by the end of the line instead of its name. The
error is reported as `missing-name` and is zero-length at the end of the line. To recover, assume
the missing token.

```oval example
measure @prop-a "Proposition A"
```

```oval counter-example
measure @prop-a
```

#### Missing Contest Kind

Occurs when a contest's name is followed by the end of the line instead of a `ContestKind`. The
error is reported as `missing-contest-kind` and is zero-length at the end of the line. To recover,
assume the missing token. The contest is kept with no kind, so the kind-specific validation rules do
not apply to it.

```oval example
contest @mayor "Mayor" single
```

```oval counter-example
contest @mayor "Mayor"
```

#### Missing Value

Occurs when a line ends where a value is required: after a field keyword, where a Markdown field
requires `:`; after `oval`; after `multiple`, `seats`, `max`, or `min`; after `as`; after a `,` in a
string list; or after a measure's `recommend`. The error is reported as `missing-value` and is
zero-length at the end of the line. To recover, assume the missing token; the field is kept with no
value.

```oval example
subtitle "Vote for one"
```

```oval counter-example
subtitle
```

#### Missing Body

Occurs when a Markdown field's line ends with `:` and the next line with content is not indented
more than it, so the lexer produced no `MarkdownBlock`. The error is reported as `missing-body` and
is zero-length after the line terminator. To recover, assume the missing token; the field is kept
with an empty body. Omit the field instead.

```oval example
note:
  Always double check the options printed on your own ballot.
```

```oval counter-example
note:
date 2026-11-03
```

#### Missing Block

Occurs when a construct whose block is required is not followed by an indented line: a `ballot`,
`section`, `contest`, or `measure`; a `campaign` with no member on its line; or a `recommend`
without `no-position` or `abstain`. The error is reported as `missing-block` and is zero-length
after the line terminator. To recover, assume the missing token; the construct is kept with an empty
block, and validation reports what the block lacks.

```oval example
ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
```

```oval counter-example
ballot @city "City of Ashford"
ballot @county "Franklin County"
  measure @prop-a "Proposition A"
    recommend yes
```

The first ballot is kept with no items.

#### Unexpected Block

Occurs when a line that takes no block is followed by one: a field line other than a Markdown
field's, a `party` reference, a `support` or `oppose` line, or a version marker. The error is
reported as `unexpected-block` and spans the block. To recover, skip the entry, which here is the
block alone; the line before it is kept.

```oval example
candidate @cfox "Carl Fox"
  designation "Small Business Owner"
```

```oval counter-example
candidate @cfox "Carl Fox"
  designation "Small Business Owner"
    website https://fox.example
```

#### Unexpected Entry

Occurs when a line begins with a token that cannot begin any entry the enclosing block accepts: a
keyword that belongs elsewhere, such as `abbreviation` inside a ballot or `campaign` inside a
measure; an identifier where no member is allowed; or a string. The error is reported as
`unexpected-entry` and spans the line and its block, if any. To recover, skip the entry. The
diagnostic names the entries the block accepts.

A `Requirement` or `Extensibility` belongs to the entry it begins, so the same error occurs when a
line begins with `required`, `optional`, `extensible`, or `exhaustive` and the rest of the line does
not continue into an entry the block accepts: a modifier alone on its line, a modifier before any
other keyword (`required campaign`), two of the same kind (`optional required contest`), the two
kinds in the wrong order (`extensible required contest`), an extensibility before a measure
(`extensible measure`), or a modifier before a construct the block does not accept
(`required section` inside a section).

```oval example
party @dem "Democratic"
  abbreviation "D"
```

```oval counter-example
party @dem "Democratic"
  designation "D"
```

```oval example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  recommend no-position
```

```oval counter-example
contest @mayor "Mayor" single
  required campaign @bsantos "Beatriz Santos"
  recommend no-position
```

A requirement applies to questions and their containers, not to campaigns.

A party campaign's block accepts only party members, `website`, and `note`, so the candidate entries
`designation` and `party` are unexpected entries there. A party member takes no role, so `as` after
one is an unexpected token.

```oval example
campaign party @green "Green Party"
  website https://greens.example
```

```oval counter-example
campaign party @green "Green Party"
  designation "Party of the Environment"
```

#### Repeated Field

Occurs when a field keyword appears a second time in the same block, or at the document level. The
error is reported as `repeated-field` and spans the second entry, including its body or block. To
recover, set the entry aside; the first occurrence stands.

```oval example
measure @prop-a "Proposition A"
  subtitle "Library parcel tax renewal"
  recommend yes
```

```oval counter-example
measure @prop-a "Proposition A"
  subtitle "Library parcel tax renewal"
  subtitle "Measure A"
  recommend yes
```

#### Repeated Flag

Occurs when `incumbent` or `withdrawn` appears twice on one `campaign` line. The error is reported
as `repeated-flag` and spans the second flag. To recover, set it aside.

```oval example
campaign incumbent withdrawn @bsantos "Beatriz Santos"
```

```oval counter-example
campaign incumbent incumbent @bsantos "Beatriz Santos"
```

#### Repeated Modifier

Occurs when `seats`, `max`, `min`, or `all` appears twice in one contest header. The error is
reported as `repeated-modifier` and spans the second occurrence, with its number. To recover, set it
aside.

```oval example
contest @board "School Board" ranked max 6 min 3
```

```oval counter-example
contest @board "School Board" ranked max 6 max 4
```

#### Misplaced Entry

Occurs when a document-level entry appears after a group that must follow it has begun: a version
marker after any other entry, a guide field after a top-level `party` or `candidate` declaration, or
a guide field or a top-level declaration after the first root ballot. The error is reported as
`misplaced-entry` and spans the entry. To recover, set the entry aside.

```oval example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03

ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
```

```oval counter-example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"

ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes

date 2026-11-03
```

#### Empty Recommendation

Occurs when a contest's `recommend` line has no `no-position` or `abstain` and its block ends
without the `support` or `oppose` line a `StanceBlock` requires. The error is reported as
`empty-recommendation` and spans the `recommend` keyword. To recover, nothing is discarded; the
block is kept, and parsing continues. Write `recommend no-position` to state that the author takes
no position, or `recommend abstain` to advise leaving the question blank.

```oval example
recommend
  support @bsantos
  justification:
    Santos has kept her 2022 promises on housing and transit.
```

```oval counter-example
recommend
  justification:
    We could not reach either candidate.
```

#### Unexpected Stance

Occurs when a contest's `recommend` line has `no-position` or `abstain` and its block contains a
`support` or `oppose` line, which a `RecommendFieldBlock` does not accept. The position is already
stated on the `recommend` line, so the block may only add `justification` and `note`. This is a case
of an unexpected entry, reported under its own identifier so that the diagnostic can name the
conflict. The error is reported as `unexpected-stance` and spans each `support` or `oppose` line. To
recover, set the entry aside. Remove either the line or the `no-position` or `abstain`. A measure
never accepts these lines, so under a measure's `recommend` they are unexpected entries instead.

```oval example
recommend abstain
  justification:
    Neither candidate answered our questionnaire.
```

```oval counter-example
recommend abstain
  support @bsantos
  justification:
    Neither candidate answered our questionnaire.
```

### 3.3. Validation Errors

A well-formed document is **valid** if it produces none of the errors below. Validation checks every
rule and reports every error it finds, in source order; it never stops at the first, with one
exception, `unsupported-version`. Each entry names the condition, the identifier, and the range the
error spans.

#### Unsupported Version

Occurs when the document has no version marker, or its marker names a major version the
implementation does not support. The error is reported as `unsupported-version` and spans the
marker, or is zero-length at the start of the first line with content when there is none. A
validator that reports it reports nothing else, since the rest of the document may not have the
meaning this specification gives it.

```oval example
oval 1
```

```oval counter-example
oval 2
```

#### Missing Guide Field

Occurs when `guide`, `election`, or `date` is absent at the document level. The error is reported as
`missing-guide-field`, once per absent field, and is zero-length at the end of the version marker's
line.

```oval example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03
```

```oval counter-example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
```

#### Empty Document

Occurs when the document has no root `Ballot`. The error is reported as `empty-document` and is
zero-length at the end of the input.

```oval example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03

ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
```

```oval counter-example
oval 1

guide "Ashford Voter Guide"
election "November 3, 2026 General Election"
date 2026-11-03
```

#### Invalid Date

Occurs when a `Date` does not exist on the proleptic Gregorian calendar: its month is not 1 through
12, its day is beyond that month's length, or it is 29 February in a year that is not a leap year.
The error is reported as `invalid-date` and spans the date.

```oval example
date 2026-11-03
```

```oval counter-example
date 2026-02-30
```

#### Undeclared Reference

Occurs when a reference names an identifier that no declaration in the document declares.
Identifiers share one namespace across every entity kind. The error is reported as
`undeclared-reference` and spans the reference.

```oval example
party @dem "Democratic"
candidate @dana-lombardi "Dana Lombardi"
  party @dem
```

```oval counter-example
candidate @dana-lombardi "Dana Lombardi"
  party @dem
```

#### Duplicate Declaration

Occurs when an identifier is declared more than once, at the top level or inline, whatever the kinds
of the declarations. The error is reported as `duplicate-declaration` and spans the identifier of
each declaration after the first in source order. References resolve to the first.

```oval example
party @dem "Democratic"
party @gop "Republican"
```

```oval counter-example
party @dem "Democratic"
party @dem "Democrats"
```

#### Mismatched Reference

Occurs when a reference resolves to a declaration of a kind its position does not accept: a `party`
line must name a party; a member of a candidate campaign must name a candidate, and a member of a
party campaign must name a party; and `support` or `oppose` must name a candidate or a party. The
error is reported as `mismatched-reference` and spans the reference.

```oval example
candidate @dana-lombardi "Dana Lombardi"
  party @dem
```

```oval counter-example
candidate @dana-lombardi "Dana Lombardi"
  party @sofia-yilmaz
```

```oval example
contest @straight "Straight Party" single
  campaign party @dem
  recommend no-position
```

```oval counter-example
contest @straight "Straight Party" single
  campaign @dem
  recommend no-position
```

The second counter-example names a party in a candidate campaign; write `campaign party`.

#### Block on a Reference

Occurs when a member that is a reference, not an inline declaration, is followed by an indented
block of candidate fields, or of party fields in a party campaign. A reference has nothing to
declare. The error is reported as `block-on-reference` and spans the block.

```oval example
campaign
  @dana-lombardi "Dana Lombardi" as "President"
    designation "United States Senator"
```

```oval counter-example
campaign
  @dana-lombardi as "President"
    designation "United States Senator"
```

#### Misplaced Candidate Field

Occurs when `designation` or `party` appears where it cannot describe a candidate: at the campaign
level of a block-form campaign, or on a one-line campaign whose member is a reference, since that
candidate is declared elsewhere. The error is reported as `misplaced-candidate-field` and spans the
field.

```oval example
campaign @cfox "Carl Fox"
  designation "Small Business Owner"
```

```oval counter-example
campaign @dana-lombardi
  designation "United States Senator"
```

#### Empty Ballot

Occurs when a `Ballot` has fields but no items. The error is reported as `empty-ballot` and spans
the ballot's header line. A ballot with no block at all is the syntax error `missing-block`.

```oval example
ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
```

```oval counter-example
ballot @city "City of Ashford"
  eligibility "Residents of the City of Ashford"
```

#### Empty Section

Occurs when a `Section` has fields but no items. The error is reported as `empty-section` and spans
the section's header line.

```oval example
section "Measures"
  measure @prop-a "Proposition A"
    recommend yes
```

```oval counter-example
section "Measures"
  note:
    There are no measures this year.
```

#### Contest Without Campaigns

Occurs when a contest whose kind is not `retention` has no campaign. The error is reported as
`contest-without-campaigns` and spans the contest's header line. A retention contest's campaign
count is governed by `retention-campaign-count`.

```oval example
contest @mayor "Mayor" single
  campaign @cfox "Carl Fox"
  recommend no-position
```

```oval counter-example
contest @mayor "Mayor" single
  recommend no-position
```

#### Campaign Without Members

Occurs when a block-form campaign, of either kind, has fields but no member. Without a member there
is nothing for a recommendation to name. The error is reported as `campaign-without-members` and
spans the campaign's line.

```oval example
campaign
  @cfox "Carl Fox"
  website https://fox.example
```

```oval counter-example
campaign
  website https://fox.example
```

#### Duplicate Member

Occurs when a candidate or party appears twice among one campaign's members. The error is reported
as `duplicate-member` and spans the later member.

```oval example
campaign
  @hana-nakamura "Hana Nakamura" as "President"
  @hiro-eriksen "Hiro Eriksen" as "Vice President"
```

```oval counter-example
campaign
  @hana-nakamura "Hana Nakamura" as "President"
  @hana-nakamura as "Vice President"
```

#### Member in Two Campaigns

Occurs when a candidate or party is a member of more than one campaign in the same contest. The
error is reported as `member-in-two-campaigns` and spans the later member.

```oval example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend no-position
```

```oval counter-example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  campaign @bsantos
  recommend no-position
```

#### Party in Two Campaigns

Occurs when a member of a candidate campaign has a `party`, and that party is a member of another
campaign in the same contest. The ballot would then offer both the candidate and their party as
separate choices for one question, which almost always means the contest was transcribed wrong. The
error is reported as `party-in-two-campaigns` and spans whichever of the two members comes later in
the source.

```oval example
contest @assembly "National Assembly" single
  campaign party @dem
  campaign party @green
  recommend no-position
```

```oval counter-example
contest @assembly "National Assembly" single
  campaign party @dem
  campaign @nreyes
  recommend no-position
```

Here `@nreyes` is declared elsewhere with `party @dem`.

#### Invalid Multiple Count

Occurs when the `Integer` after `multiple` is less than 2. The error is reported as
`invalid-multiple-count` and spans the integer.

```oval example
contest @council "City Council" multiple 3
```

```oval counter-example
contest @council "City Council" multiple 1
```

#### Invalid Ranked Max

Occurs when the `Integer` after `max` is 0. The error is reported as `invalid-ranked-max` and spans
the integer.

```oval example
contest @board "School Board" ranked max 3
```

```oval counter-example
contest @board "School Board" ranked max 0
```

#### Invalid Seat Count

Occurs when the `Integer` after `seats` is 0. The error is reported as `invalid-seat-count` and
spans the integer.

```oval example
contest @council "City Council" ranked seats 3
```

```oval counter-example
contest @council "City Council" ranked seats 0
```

#### Invalid Minimum

Occurs when the `Integer` after `min` is 0, exceeds the `max` of a `ranked` contest, or exceeds the
number of votes of a `multiple` contest. A ballot could then never meet the minimum. The error is
reported as `invalid-minimum` and spans the integer.

```oval example
contest @board "School Board" multiple 3 min 2
```

```oval counter-example
contest @board "School Board" multiple 3 min 4
```

#### Ranked All With a Limit

Occurs when `all` appears in a header with `max` or `min`. Ranking every printed choice leaves no
room for either limit. The error is reported as `ranked-all-with-limit` and spans `all`.

```oval example
contest @house "House of Representatives" ranked all
```

```oval counter-example
contest @house "House of Representatives" ranked all max 6
```

#### Retention Campaign Count

Occurs when a `retention` contest has no campaign or more than one. The error is reported as
`retention-campaign-count` and spans the contest's header line when there is no campaign, or each
campaign after the first.

```oval example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent @ueda "Mika Ueda"
  recommend
    support @ueda
```

```oval counter-example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent @ueda "Mika Ueda"
  campaign @cfox "Carl Fox"
  recommend
    support @ueda
```

#### Party Campaign in a Retention Contest

Occurs when a `retention` contest's campaign is a party campaign. A retention question asks for a
yes or no on one candidate, so its campaign names a candidate. The error is reported as
`retention-party-campaign` and spans the campaign's line.

```oval example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent @ueda "Mika Ueda"
  recommend no-position
```

```oval counter-example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent party @dem
  recommend no-position
```

#### Missing Recommendation

Occurs when a `Contest` has no `ContestRecommend` or a `Measure` has no `MeasureRecommend`. Every
question on the ballot carries the author's position, even when that position is `no-position` or
`abstain`. The error is reported as `missing-recommendation` and spans the contest's or measure's
header line.

```oval example
measure @prop-a "Proposition A"
  recommend no-position
```

```oval counter-example
measure @prop-a "Proposition A"
  subtitle "Library parcel tax renewal"
```

#### Repeated Recommendation

Occurs when a `Contest` or `Measure` has more than one recommendation. The error is reported as
`repeated-recommendation` and spans each `recommend` entry after the first.

```oval example
measure @prop-a "Proposition A"
  recommend yes
```

```oval counter-example
measure @prop-a "Proposition A"
  recommend yes
  recommend no
```

#### Campaign Not in Contest

Occurs when a `support` or `oppose` reference names a candidate or party that is a member of no
campaign in the enclosing contest. A recommendation names a campaign by one of its members, and a
candidate's `party` field is not membership. The error is reported as `campaign-not-in-contest` and
spans the reference.

```oval example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  recommend
    support @bsantos
```

```oval counter-example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  recommend
    support @cfox
```

#### Repeated Oppose

Occurs when a `StanceBlock` has more than one `oppose` line. `oppose` is unordered, so one line
lists every campaign the author advises against. The error is reported as `repeated-oppose` and
spans each `oppose` line after the first.

```oval example
recommend
  support @bsantos
  oppose @cfox @dwright
```

```oval counter-example
recommend
  support @bsantos
  oppose @cfox
  oppose @dwright
```

#### Repeated Stance

Occurs when the same choice appears more than once across a recommendation's `support` lines and its
`oppose` line. Two choices are the same when they are references that resolve to the same campaign,
even if they name different members; when they are write-ins with identical names; or when both are
bare write-ins. The error is reported as `repeated-stance` and spans each choice after the first.

```oval example
recommend
  support @bsantos
  oppose @cfox
```

```oval counter-example
recommend
  support @bsantos
  oppose @bsantos
```

```oval example
recommend
  support write-in "Jane Doe"
  oppose write-in
```

```oval counter-example
recommend
  support write-in "Jane Doe"
  oppose write-in "Jane Doe"
```

#### Tiers in a Single Contest

Occurs when a `single` contest's recommendation has more than one `support` line. A vote-for-one
contest has no second choice. The error is reported as `tiers-in-single` and spans each `support`
line after the first.

```oval example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @bsantos
```

```oval counter-example
contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @bsantos
    support @cfox
```

#### Support Beyond Ranked Max

Occurs when a `ranked` contest declares `max` _n_ and its recommendation's `support` lines name more
than _n_ choices in total, counting each write-in as one. The error is reported as
`support-over-max` and spans the first choice beyond the limit.

```oval example
contest @board "School Board" ranked max 2
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor
    support @bsantos
```

```oval counter-example
contest @board "School Board" ranked max 2
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor
    support @bsantos
    support @cfox
```

#### Retention Stance

Occurs when a `retention` contest's recommendation is anything other than one `support` line naming
only its campaign, one `oppose` line naming only its campaign, `recommend no-position`, or
`recommend abstain`. A retention question has a single answer, yes or no. The error is reported as
`retention-stance` and spans the line that breaks the shape.

```oval example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent @ueda "Mika Ueda"
  recommend
    oppose @ueda
```

```oval counter-example
contest @judge-ueda "Retain Judge Ueda?" retention
  campaign incumbent @ueda "Mika Ueda"
  recommend
    support @ueda
    oppose @ueda
```

#### Abstain on a Required Question

Occurs when a contest's or measure's recommendation is `abstain` and the question's effective
requirement is required, whether the requirement is written on the question or inherited from its
section or ballots. A required question cannot be left blank. The error is reported as
`abstain-on-required` and spans the `abstain` keyword.

```oval example
required section "City Measures"
  measure @prop-a "Proposition A"
    recommend yes
  optional measure @prop-b "Proposition B"
    recommend abstain
```

```oval counter-example
required section "City Measures"
  measure @prop-a "Proposition A"
    recommend yes
  measure @prop-b "Proposition B"
    recommend abstain
```

#### Write-in Not Accepted

Occurs when a recommendation contains a `write-in` and its contest is exhaustive: nothing makes it
extensible, a nearer `exhaustive` overrides an inherited `extensible`, or it is a `retention`
contest. The error is reported as `write-in-not-accepted` and spans the write-in.

```oval example
extensible section "City Offices"
  contest @mayor "Mayor" single
    campaign @bsantos "Beatriz Santos"
    recommend
      support write-in "Jane Doe"
```

```oval counter-example
extensible section "City Offices"
  exhaustive contest @mayor "Mayor" single
    campaign @bsantos "Beatriz Santos"
    recommend
      support write-in "Jane Doe"
```

#### Extensible Retention Contest

Occurs when `extensible` is written before a `retention` contest. A retention question is answered
yes or no, so there is nothing to write in. The error is reported as `extensible-retention` and
spans the `extensible` keyword. An `extensible` inherited from a section or ballot is ignored
instead.

```oval example
extensible section "Judicial"
  contest @judge-ueda "Retain Judge Ueda?" retention
    campaign incumbent @ueda "Mika Ueda"
    recommend
      support @ueda
```

```oval counter-example
section "Judicial"
  extensible contest @judge-ueda "Retain Judge Ueda?" retention
    campaign incumbent @ueda "Mika Ueda"
    recommend
      support @ueda
```

#### Unsupported Markdown

Occurs when a `MarkdownBlock` value, parsed as described in Section 2.2 (Markdown Profile), contains
a construct outside the profile, or a link whose destination is not an absolute `http` or `https`
URL. The error is reported as `markdown-profile`, once per construct, and spans the construct within
the body. Checking this rule requires a CommonMark parser with the tables extension; no other rule
does.

```oval example
note:
  See the [full text](https://ashford.example/prop-a) before voting.
```

```oval counter-example
note:
  See the full text at <https://ashford.example/prop-a>:

      ashford.example/prop-a
```

The second example contains an indented code block.

### 3.4. Warnings

A warning marks a document that is valid but probably not what its author intends: something is
unused, duplicated, unbalanced, or empty in a way the grammar and the errors above allow. Warnings
do not affect validity. A validator reports every warning with the same identifiers and ranges as
errors, and a compiler compiles a document that has warnings. A tool may let an author silence a
warning; this specification does not say how.

#### Unused Declaration

Occurs when a top-level `Party` declaration is named by no `party` line and is a member of no
campaign, or a top-level `Candidate` declaration is a member of no campaign. Inline declarations are
used by the construct that declares them and are never reported. The warning is reported as
`unused-declaration` and spans the declaration's header line.

```oval example
party @dem "Democratic"
candidate @dana-lombardi "Dana Lombardi"
  party @dem
```

```oval counter-example
party @dem "Democratic"
party @gop "Republican"
candidate @dana-lombardi "Dana Lombardi"
  party @dem
```

#### Duplicate Party Name

Occurs when two `Party` declarations have the same name and different identifiers. The warning is
reported as `duplicate-party-name` and spans the later declaration's name.

```oval example
party @dem "Democratic"
party @gop "Republican"
```

```oval counter-example
party @dem "Democratic"
party @dems "Democratic"
```

#### Duplicate Ballot Title

Occurs when two root ballots, or two sibling sub-ballots, have identical titles. The warning is
reported as `duplicate-ballot-title` and spans the later title.

```oval example
ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
ballot @county "Franklin County"
  measure @prop-b "Proposition B"
    recommend no
```

```oval counter-example
ballot @city "City of Ashford"
  measure @prop-a "Proposition A"
    recommend yes
ballot @city-2 "City of Ashford"
  measure @prop-b "Proposition B"
    recommend no
```

#### Support Beyond Multiple Count

Occurs when a `multiple` contest gives the voter _n_ votes and its recommendation's `support` lines
name more than _n_ choices in total, counting each write-in as one. The count is against votes, not
seats: in limited voting there are more seats than votes. The author may mean to rank more
acceptable candidates than there are votes, so this is a warning rather than an error. It is
reported as `support-over-count` and spans the first choice beyond _n_.

```oval example
contest @council "City Council" multiple 2 seats 3
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor @bsantos
```

```oval counter-example
contest @council "City Council" multiple 2 seats 3
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor @bsantos
    support @cfox
```

#### Support Below the Minimum

Occurs when a recommendation in a `multiple` or `ranked` contest has a `support` line and its
`support` lines name fewer choices in total than the contest's `min`, or, in a `ranked all` contest,
fewer than its campaigns, counting each write-in as one. A reader who marks only those choices casts
a ballot that does not count. Advice such as "rank Okafor first, then choose for yourself" is
legitimate, so this is a warning rather than an error. A recommendation with no `support` line is
not checked. The warning is reported as `support-below-minimum` and spans the last `support` line.

```oval example
contest @board "School Board" ranked min 2
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor
    support @bsantos
```

```oval counter-example
contest @board "School Board" ranked min 2
  campaign @aokafor "Ada Okafor"
  campaign @bsantos "Beatriz Santos"
  campaign @cfox "Carl Fox"
  recommend
    support @aokafor
```

#### Write-in on the Ballot

Occurs when a named write-in is identical to the name of a candidate who is a member of one of the
contest's campaigns. That person is already printed on the ballot, so writing them in is at best
redundant; reference their campaign instead. The warning is reported as `write-in-on-ballot` and
spans the name.

```oval example
extensible contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  recommend
    support @bsantos
```

```oval counter-example
extensible contest @mayor "Mayor" single
  campaign @bsantos "Beatriz Santos"
  recommend
    support write-in "Beatriz Santos"
```

#### Unpaired Outcome

Occurs when a `Measure` has exactly one of `if_yes` and `if_no`. The warning is reported as
`unpaired-outcome` and spans the field that is present.

```oval example
measure @prop-a "Proposition A"
  if_yes:
    The library parcel tax continues at the same rate.
  if_no:
    The library parcel tax expires in 2027.
  recommend yes
```

```oval counter-example
measure @prop-a "Proposition A"
  if_yes:
    The library parcel tax continues at the same rate.
  recommend yes
```

#### Insecure URL

Occurs when a `Url`, or a link destination inside a `MarkdownBlock`, uses the `http` scheme rather
than `https`. The URL is valid, but a reader following it is not protected from tampering, and most
sites that still answer on `http` redirect to `https`. The warning is reported as `insecure-url` and
spans the URL, or the link destination within the body.

```oval example
website https://santosformayor.example
```

```oval counter-example
website http://santosformayor.example
```

#### Empty String

Occurs when a `String` contains no characters, or only spaces and tabs. A string is display text, so
an empty one shows the reader a blank where a title, a name, or a value belongs. Write the text, or
omit an optional field instead of leaving it empty. The warning is reported as `empty-string` and
spans the string, its quotes included.

```oval example
subtitle "Vote for one"
```

```oval counter-example
subtitle ""
```

---

## 4. Conformance

The specification declares four levels of conformance, so tools can correctly advertise to consumers
what parts of the language they implement. Each level includes the requirements of the previous
level.

- **Lexer-conformant.** Tokenizes any input per Sections 2.2–2.4, producing tokens that cover every
  byte, and reports lexical errors with the ranges and recovery of Section 3.1. Sufficient for
  syntax highlighting.
- **Parser-conformant.** Accepts exactly the well-formed documents of Section 2 and produces a tree
  that covers every byte of the source, with the recoveries of Section 3.2. Sufficient for a
  formatter.
- **Validator-conformant.** Parser-conformant, and reports every Section 3 rule with its identifier
  and severity.
- **Compiler-conformant.** Validator-conformant, and for every valid document produces the canonical
  output of Section 5, byte-for-byte equal to the reference implementation's after JSON
  normalization (key order and whitespace disregarded).

A conformance claim names the major version of the language and the edition of this specification it
implements. An implementation reports documents of another major version under `unsupported-version`
rather than attempting to process them.

A conformance test suite (not yet written) will consist of well-formed and malformed documents, and
for each well-formed document the expected validation report and canonical resolved output.

---

## 5. Canonical Output

Compiling a valid document produces one JSON value: the data model with every reference resolved,
plus derived fields that any renderer would otherwise recompute. Its shape is fixed by
`oval.schema.json`, a JSON Schema (draft 2020-12) published beside this document and as the npm
package `@oval-vote/oval-schema`, which is normative. This section explains the conventions the
schema follows, lays out the shape it fixes, and defines the derived fields. The output is called
_canonical_ because two conformant implementations must produce the same value for the same
document; it is a contract between implementations and consumers, not a document for people to read.

The output is the semantic model, not the syntax tree. It carries no source positions, comments, or
whitespace, so two parsers cannot disagree about it on formatting. Tools that need positions use the
syntax tree or the diagnostics.

### 5.1. Conventions

- **Keys mirror the language.** Every field that comes from a keyword uses that keyword's exact
  spelling, such as `if_yes` and `fiscal_impact`, and every value that comes from a keyword keeps
  its spelling too, such as `"no-position"`. Derived fields use the same `snake_case` style. The one
  exception is the contest's voting method, which the language writes on the header line without a
  keyword; it is the `method` object, because `kind` is the discriminator used by every item object.
- **Arrays, not maps.** Order is meaning in Oval, so parties, candidates, ballots, items, campaigns,
  members, and tiers are arrays in document or ballot order. Entities carry an `id`, and references
  are identifier strings without the leading `@`.
- **Absent fields are omitted.** An optional field the author did not write is not present; it is
  never `null`. `supporters` and `opponents` therefore have three states: omitted (not researched),
  `[]` (researched, none), or a list.
- **Booleans are always present.** `incumbent`, `withdrawn`, `required`, `extensible`, and
  `complete` are written even when false.
- **The root echoes the version marker.** `oval` is the document's declared major version as an
  integer, `1`. Later editions of the same major version only add keys and enumerated values to the
  output, so a consumer written to an earlier edition reads the output of a later one.
- **Discriminated unions use `kind`.** Every item in a ballot or section has a `kind` of `section`,
  `contest`, `measure`, or `ballot`, and root ballots carry `kind: "ballot"` too, so one type
  describes a ballot anywhere. `method.kind`, `campaign.kind`, `member.kind`, `choice.kind`, and
  `stance.position` discriminate the same way.
- **Markdown is a string** with common indentation removed, terminators normalized to `\n`, and no
  trailing terminator. It is within the Markdown profile. Rendering it is the consumer's job.
- **Campaigns are referred to by index** within their contest's `campaigns` array, since they have
  no identifiers.
- **Strict producers, tolerant consumers.** The schema forbids unknown keys so that a producer's
  mistakes are caught. Consumers should ignore keys they do not recognize, and tolerate values of an
  enumeration they do not recognize, because a later edition of this specification may add fields
  and values. Removing or renaming a field or a value takes a new major version.

### 5.2. Shape

This section is generated from `packages/oval-schema/oval.schema.json` by `.scripts/shape.mjs` and
lists the objects and keys the schema defines. Every value is one of the objects below, an array of
them, or a scalar. An _identifier_ is an entity's identifier without its leading `@`, and _Markdown_
is a body with its common indentation removed, line terminators normalized to `\n`, and no trailing
terminator. A key whose meaning begins with "Derived" is computed from the document.

#### Root

The canonical output of compiling an Oval document: the data model with every reference resolved,
plus derived fields for renderers. Keys mirror the language's keywords.

| Key          | Type                    | Required | Meaning                                                                             |
| ------------ | ----------------------- | -------- | ----------------------------------------------------------------------------------- |
| `oval`       | integer, at least 1     | yes      | The major version of the Oval language the document declares in its version marker. |
| `guide`      | Guide                   | yes      | The document-level fields.                                                          |
| `parties`    | array of Party          | yes      | Every party declared at the top level or inline, in document order.                 |
| `candidates` | array of Candidate      | yes      | Every candidate declared at the top level or inline, in document order.             |
| `ballots`    | array of Ballot         | yes      | Root ballots, in presentation order.                                                |
| `questions`  | array of Question Entry | yes      | Derived. Every contest and measure in ballot order, with where it sits in the tree. |
| `counts`     | Counts                  | yes      | Derived. Totals across the whole document.                                          |

#### Guide

The document-level fields: what the guide is and which election it covers.

| Key        | Type                 | Required | Meaning                                               |
| ---------- | -------------------- | -------- | ----------------------------------------------------- |
| `name`     | string               | yes      | The guide's title, from `guide`.                      |
| `election` | string               | yes      | The election as named on the ballot, from `election`. |
| `date`     | string, `YYYY-MM-DD` | yes      | Election day as `YYYY-MM-DD`, from `date`.            |
| `note`     | Markdown             | no       | The guide's introductory note, from `note`.           |

#### Party

A political party.

| Key            | Type                | Required | Meaning                                                                                         |
| -------------- | ------------------- | -------- | ----------------------------------------------------------------------------------------------- |
| `id`           | identifier          | yes      | The party's identifier.                                                                         |
| `name`         | string              | yes      | The party's name as printed on the ballot.                                                      |
| `abbreviation` | string              | no       | The short form printed beside candidates, from `abbreviation`.                                  |
| `website`      | string, a URL       | no       | The party's website, from `website`.                                                            |
| `note`         | Markdown            | no       | The author's note on the party, from `note`.                                                    |
| `candidates`   | array of identifier | yes      | Derived. Identifiers of every candidate whose party this is, in document order.                 |
| `contests`     | array of identifier | yes      | Derived. Identifiers of every contest in which the party is a campaign member, in ballot order. |

#### Candidate

A person who appears in one or more campaigns.

| Key           | Type                | Required | Meaning                                                                                             |
| ------------- | ------------------- | -------- | --------------------------------------------------------------------------------------------------- |
| `id`          | identifier          | yes      | The candidate's identifier.                                                                         |
| `name`        | string              | yes      | The candidate's name as printed on the ballot.                                                      |
| `designation` | string              | no       | The occupation or title printed beside the name, from `designation`.                                |
| `party`       | identifier          | no       | Identifier of the candidate's party, from `party`.                                                  |
| `note`        | Markdown            | no       | The author's note on the candidate, from `note`.                                                    |
| `contests`    | array of identifier | yes      | Derived. Identifiers of every contest in which the candidate is a campaign member, in ballot order. |

#### Ballot

A ballot: a root ballot or a sub-ballot for a narrower electorate.

| Key           | Type                 | Required | Meaning                                                                                                   |
| ------------- | -------------------- | -------- | --------------------------------------------------------------------------------------------------------- |
| `kind`        | `"ballot"`           | yes      | Always `ballot`.                                                                                          |
| `id`          | identifier           | yes      | The ballot's identifier.                                                                                  |
| `title`       | string               | yes      | The jurisdiction or grouping the ballot covers.                                                           |
| `eligibility` | string               | no       | Who receives this ballot, from `eligibility`.                                                             |
| `note`        | Markdown             | no       | The author's note on the ballot, from `note`.                                                             |
| `path`        | array of identifier  | yes      | Derived. Identifiers of the enclosing ballots, outermost first, not including this one. Empty for a root. |
| `items`       | array of ballot_item | yes      | Sections, contests, measures, and sub-ballots in ballot order.                                            |

#### Section

A titled group of contests and measures within a ballot.

| Key     | Type                        | Required | Meaning                                        |
| ------- | --------------------------- | -------- | ---------------------------------------------- |
| `kind`  | `"section"`                 | yes      | Always `section`.                              |
| `title` | string                      | yes      | The section's title as printed on the ballot.  |
| `note`  | Markdown                    | no       | The author's note on the section, from `note`. |
| `items` | array of Contest or Measure | yes      | Contests and measures in ballot order.         |

#### Contest

A race between campaigns for an office.

| Key              | Type                   | Required | Meaning                                                                                                         |
| ---------------- | ---------------------- | -------- | --------------------------------------------------------------------------------------------------------------- |
| `kind`           | `"contest"`            | yes      | Always `contest`.                                                                                               |
| `id`             | identifier             | yes      | The contest's identifier.                                                                                       |
| `title`          | string                 | yes      | The contest as named on the ballot.                                                                             |
| `subtitle`       | string                 | no       | A line printed under the title, from `subtitle`.                                                                |
| `method`         | Method                 | yes      | The voting method, from the contest header.                                                                     |
| `tabulation`     | string                 | no       | How votes are counted, from `tabulation`.                                                                       |
| `note`           | Markdown               | no       | The author's note on the contest, from `note`.                                                                  |
| `location`       | Location               | yes      | Derived. Where the contest sits in the ballot tree.                                                             |
| `required`       | boolean                | yes      | Derived. Whether the voter must answer it, from the nearest `required` or `optional`; false if none.            |
| `extensible`     | boolean                | yes      | Derived. Whether the voter may write in a choice, from the nearest `extensible` or `exhaustive`; false if none. |
| `campaigns`      | array of Campaign      | yes      | In ballot order. A campaign is referred to by its index in this array.                                          |
| `recommendation` | Contest Recommendation | yes      | The author's recommendation on the contest.                                                                     |

#### Method

The voting method, from the contest header.

| Key        | Type                                                       | Required                 | Meaning                                                                                                                                     |
| ---------- | ---------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `kind`     | one of `"single"`, `"multiple"`, `"ranked"`, `"retention"` | yes                      | One of `single`, `multiple`, `ranked`, or `retention`.                                                                                      |
| `seats`    | integer, at least 1                                        | with `"single"`          | How many candidates are elected, from `seats`; 1 if not written.                                                                            |
| `votes`    | integer, at least 2                                        | with `"multiple"`        | How many choices a voter may mark, from `multiple`.                                                                                         |
| `seats`    | integer, at least 1                                        | with `"multiple"`        | How many candidates are elected, from `seats`; the number of votes if not written.                                                          |
| `min`      | integer, at least 1                                        | with `"multiple"`        | How many choices a ballot that marks the contest must mark, from `min`; 1 if not written.                                                   |
| `seats`    | integer, at least 1                                        | with `"ranked"`          | How many candidates are elected, from `seats`; 1 if not written.                                                                            |
| `max`      | integer, at least 1                                        | optional with `"ranked"` | How many choices a voter may rank, from `max`. Omitted when unlimited.                                                                      |
| `min`      | integer, at least 1                                        | with `"ranked"`          | How many choices a ballot that ranks the contest must rank, from `min`; 1 if not written. Derived as the number of campaigns when complete. |
| `complete` | boolean                                                    | with `"ranked"`          | Whether every printed choice must be ranked, from `all`.                                                                                    |

#### Campaign

One or more candidates, or one or more parties, running together for a contest.

| Key         | Type                               | Required | Meaning                                                                                          |
| ----------- | ---------------------------------- | -------- | ------------------------------------------------------------------------------------------------ |
| `index`     | integer, at least 0                | yes      | Derived. Position in the contest's campaigns array.                                              |
| `kind`      | one of `"candidates"`, `"parties"` | yes      | Whether the members are candidates or parties, from `campaign` or `campaign party`.              |
| `label`     | string                             | yes      | Derived. The members' names joined with " / ", head first.                                       |
| `head`      | identifier                         | yes      | Derived. Identifier of the first member, a candidate or a party according to `kind`.             |
| `party`     | identifier                         | no       | Derived. For a candidate campaign, the party shared by every member, when all have the same one. |
| `members`   | array of Member                    | yes      | The campaign's candidates or parties, head first.                                                |
| `incumbent` | boolean                            | yes      | Whether the campaign currently occupies the office, from the `incumbent` flag.                   |
| `withdrawn` | boolean                            | yes      | Whether the campaign has ended but remains on the ballot, from the `withdrawn` flag.             |
| `website`   | string, a URL                      | no       | The campaign's website, from `website`.                                                          |
| `note`      | Markdown                           | no       | The author's note on the campaign, from `note`.                                                  |
| `stance`    | Stance                             | yes      | Derived. What the contest's recommendation says about this campaign.                             |

#### Member

A candidate's or party's place on a campaign.

| Key         | Type                            | Required                    | Meaning                                                        |
| ----------- | ------------------------------- | --------------------------- | -------------------------------------------------------------- |
| `kind`      | one of `"candidate"`, `"party"` | yes                         | One of `candidate` or `party`, matching the campaign's `kind`. |
| `candidate` | identifier                      | with `"candidate"`          | The candidate's identifier.                                    |
| `role`      | string                          | optional with `"candidate"` | The member's role on the ticket, from `as`.                    |
| `note`      | Markdown                        | optional with `"candidate"` | The author's note on this member, from the member's `note`.    |
| `party`     | identifier                      | with `"party"`              | The party's identifier.                                        |

#### Stance

Derived. What the contest's recommendation says about this campaign.

| Key        | Type                                     | Required         | Meaning                                                       |
| ---------- | ---------------------------------------- | ---------------- | ------------------------------------------------------------- |
| `position` | one of `"support"`, `"oppose"`, `"none"` | yes              | One of `support`, `oppose`, or `none`.                        |
| `tier`     | integer, at least 0                      | with `"support"` | Zero-based index of the support tier the campaign appears in. |

#### Contest Recommendation

The author's recommendation on a contest.

| Key             | Type                                            | Required | Meaning                                                                                                                               |
| --------------- | ----------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `position`      | one of `"stance"`, `"no-position"`, `"abstain"` | yes      | The kind of recommendation: `stance` when it has `support` or `oppose` lines, otherwise `no-position` or `abstain`, from `recommend`. |
| `support`       | array of arrays of Choice                       | yes      | Tiers as written, most preferred tier first. Empty unless position is stance.                                                         |
| `oppose`        | array of Choice                                 | yes      | Choices as written. Empty unless position is stance.                                                                                  |
| `justification` | Markdown                                        | no       | The author's reasoning, from `justification`.                                                                                         |
| `note`          | Markdown                                        | no       | The author's note on the recommendation, from `note`.                                                                                 |

#### Choice

One choice in a recommendation: a campaign printed on the ballot, or a write-in.

| Key         | Type                                          | Required                   | Meaning                                                                    |
| ----------- | --------------------------------------------- | -------------------------- | -------------------------------------------------------------------------- |
| `kind`      | one of `"candidate"`, `"party"`, `"write_in"` | yes                        | One of `candidate`, `party`, or `write_in`.                                |
| `candidate` | identifier                                    | with `"candidate"`         | Derived. Identifier of the head of the campaign the reference resolves to. |
| `campaign`  | integer, at least 0                           | with `"candidate"`         | Derived. Index of the campaign the candidate resolves to.                  |
| `party`     | identifier                                    | with `"party"`             | Derived. Identifier of the head of the campaign the reference resolves to. |
| `campaign`  | integer, at least 0                           | with `"party"`             | Derived. Index of the campaign the party resolves to.                      |
| `name`      | string                                        | optional with `"write_in"` | The name to write in, from `write-in`. Omitted for a bare write-in.        |

#### Measure

A ballot measure: a question answered yes or no.

| Key              | Type                   | Required | Meaning                                                                                                                     |
| ---------------- | ---------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------- |
| `description`    | Markdown               | no       | The question as printed on the ballot, from `description`.                                                                  |
| `kind`           | `"measure"`            | yes      | Always `measure`.                                                                                                           |
| `id`             | identifier             | yes      | The measure's identifier.                                                                                                   |
| `title`          | string                 | yes      | The measure as named on the ballot.                                                                                         |
| `subtitle`       | string                 | no       | A line printed under the title, from `subtitle`.                                                                            |
| `if_yes`         | Markdown               | no       | What a yes vote does, from `if_yes`.                                                                                        |
| `if_no`          | Markdown               | no       | What a no vote does, from `if_no`.                                                                                          |
| `placement`      | string                 | no       | How the measure reached the ballot, from `placement`.                                                                       |
| `tabulation`     | string                 | no       | What it takes to pass, from `tabulation`.                                                                                   |
| `fiscal_impact`  | string                 | no       | The measure's fiscal impact, from `fiscal_impact`.                                                                          |
| `supporters`     | array of string        | no       | Organizations supporting the measure, from `supporters`. Omitted when not researched; empty when researched and none known. |
| `opponents`      | array of string        | no       | Organizations opposing the measure, from `opponents`. Omitted when not researched; empty when researched and none known.    |
| `note`           | Markdown               | no       | The author's note on the measure, from `note`.                                                                              |
| `location`       | Location               | yes      | Derived. Where the measure sits in the ballot tree.                                                                         |
| `required`       | boolean                | yes      | Derived. Whether the voter must answer it, from the nearest `required` or `optional`; false if none.                        |
| `recommendation` | Measure Recommendation | yes      | The author's recommendation on the measure.                                                                                 |

#### Measure Recommendation

The author's recommendation on a measure.

| Key             | Type                                                 | Required | Meaning                                               |
| --------------- | ---------------------------------------------------- | -------- | ----------------------------------------------------- |
| `position`      | one of `"yes"`, `"no"`, `"no-position"`, `"abstain"` | yes      | The recommended vote, from `recommend`.               |
| `justification` | Markdown                                             | no       | The author's reasoning, from `justification`.         |
| `note`          | Markdown                                             | no       | The author's note on the recommendation, from `note`. |

#### Question Entry

Derived. One contest or measure in the document's flat index of questions.

| Key        | Type                            | Required | Meaning                                                                    |
| ---------- | ------------------------------- | -------- | -------------------------------------------------------------------------- |
| `kind`     | one of `"contest"`, `"measure"` | yes      | Whether the question is a `contest` or a `measure`.                        |
| `id`       | identifier                      | yes      | The question's identifier.                                                 |
| `title`    | string                          | yes      | The question's title.                                                      |
| `location` | Location                        | yes      | Where the question sits in the ballot tree.                                |
| `required` | boolean                         | yes      | Whether the voter must answer this question, as on the contest or measure. |

#### Location

Derived. Where a question sits in the ballot tree.

| Key       | Type                | Required | Meaning                                                                       |
| --------- | ------------------- | -------- | ----------------------------------------------------------------------------- |
| `ballot`  | identifier          | yes      | Identifier of the innermost enclosing ballot.                                 |
| `section` | string              | no       | Title of the enclosing section, when there is one.                            |
| `path`    | array of identifier | yes      | Identifiers of every enclosing ballot, outermost first, ending with `ballot`. |

#### Counts

Derived. Totals across the whole document.

| Key          | Type                | Required | Meaning                                    |
| ------------ | ------------------- | -------- | ------------------------------------------ |
| `ballots`    | integer, at least 1 | yes      | Every ballot, roots and sub-ballots alike. |
| `sections`   | integer, at least 0 | yes      | Every section.                             |
| `contests`   | integer, at least 0 | yes      | Every contest.                             |
| `measures`   | integer, at least 0 | yes      | Every measure.                             |
| `campaigns`  | integer, at least 0 | yes      | Every campaign.                            |
| `candidates` | integer, at least 0 | yes      | Every candidate.                           |
| `parties`    | integer, at least 0 | yes      | Every party.                               |

### 5.3. Derived fields

Everything below is computed from the document; none of it can disagree with the source.

| Field                | On                               | Definition                                                                                                                                                                                 |
| -------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `candidates`         | party                            | Identifiers of every candidate whose party this is, in document order.                                                                                                                     |
| `contests`           | candidate                        | Identifiers of every contest in which the candidate is a campaign member, in ballot order.                                                                                                 |
| `contests`           | party                            | Identifiers of every contest in which the party is a campaign member, in ballot order.                                                                                                     |
| `path`               | ballot                           | Identifiers of the enclosing ballots, outermost first, excluding the ballot itself. Empty for a root.                                                                                      |
| `location`           | contest, measure, question entry | `ballot`, the innermost enclosing ballot's identifier; `section`, the enclosing section's title if any; `path`, every enclosing ballot's identifier outermost first, ending with `ballot`. |
| `required`           | contest, measure, question entry | `true` when the nearest requirement on the question, its section, or its ballots is `required`; `false` when it is `optional` or there is none.                                            |
| `extensible`         | contest                          | `true` when the nearest extensibility on the contest, its section, or its ballots is `extensible` and the contest is not `retention`.                                                      |
| `min`                | method                           | For `multiple` and `ranked`, the written `min`, or 1 when none is written; for `ranked all`, the number of campaigns.                                                                      |
| `index`              | campaign                         | Position in the contest's `campaigns` array.                                                                                                                                               |
| `head`               | campaign                         | Identifier of the first member: a candidate or a party, according to the campaign's `kind`.                                                                                                |
| `label`              | campaign                         | The members' names joined with `" / "`, head first. For a single member, its name.                                                                                                         |
| `party`              | campaign                         | For a candidate campaign, the party every member shares. Omitted otherwise, including for party campaigns.                                                                                 |
| `stance`             | campaign                         | What the contest's recommendation says about this campaign: `support` with the zero-based `tier`, `oppose`, or `none`. Unless `position` is `stance`, every campaign has `none`.           |
| `campaign`           | choice                           | The index of the campaign a candidate or party choice resolves to.                                                                                                                         |
| `candidate`, `party` | choice                           | The identifier of that campaign's head, whichever member the reference names.                                                                                                              |
| `questions`          | root                             | Every contest and measure in ballot order, each with `kind`, `id`, `title`, and `location`. A flat index for tables of contents and per-electorate views.                                  |
| `counts`             | root                             | Totals of ballots (all, not only roots), sections, contests, measures, campaigns, candidates, and parties.                                                                                 |

Resolution of candidate identifiers to campaigns follows Section 2.15. A valid document resolves
every reference, so the derived fields are always complete.

### 5.4. Example

`examples/franklin.oval.json` is the canonical output of `examples/franklin.oval`, the fictional
full-scale guide: six root ballots, forty-four contests, twenty-eight measures. It is produced by
the reference implementation (`oval compile --pretty`) and validates against the schema:

```sh
check-jsonschema --schemafile packages/oval-schema/oval.schema.json examples/franklin.oval.json
```

The repository's `mise.toml` installs `check-jsonschema`.

---

## 6. Formatting

A formatter rewrites a well-formed document into the canonical style below. Formatting never changes
meaning: the formatted document has the same significant tokens, except the redundant modifiers it
removes, the references it rewrites, and the voting-method modifiers it removes or reorders, the
same Markdown values, and compiles to the same canonical output as the original. Formatting is
idempotent.

A formatter refuses a document with lexical or syntax errors and reports them instead. Recovery from
errors produces a tree whose shape the author did not intend, and reformatting around it could move
text in surprising ways.

### 6.1. Whitespace

- Line terminators become U+000A. The document ends with exactly one.
- Trailing spaces are removed from every line, except content lines inside a `MarkdownBlock`, where
  two trailing spaces are a hard line break in Markdown and are preserved. Whitespace-only lines
  inside a body become empty.
- Every line is indented two spaces per enclosing block, whatever the source used. A `MarkdownBlock`
  body is indented one level deeper than its keyword line; each body line keeps its indentation
  beyond the body's common minimum, so lists and continuation lines keep their shape.
- Tokens on a line are separated by one space. `:` and `,` attach to the token before them, and `@`
  to the word after it. A trailing comment is preceded by one space.

### 6.2. Blank lines and comments

- Runs of blank lines collapse to one. Blank lines inside a `MarkdownBlock` body are content and are
  kept exactly.
- A blank line and any comment lines after it belong to the next line with tokens: they are indented
  to that line's depth and move with it when entries are reordered. A blank line the author wrote at
  the start of a block stays; one that would begin a block only because its entry was reordered is
  removed. Blank lines at the start and end of the document are removed. A formatter never inserts a
  blank line. Comments after the last line of the document are placed at column zero.

This attachment rule is a formatting decision, not a parsing one. The parser attaches a comment to
whichever node is open when the next significant token is consumed, which puts a comment just before
an indented block in the parent and a comment just before a dedent in the inner block. Neither is
where a reader would say the comment belongs, so the formatter corrects both.

### 6.3. Entry order

Within a block, entries are grouped by kind in the order Appendix A lists the block's alternatives,
and kept in source order within a group:

| Block                                  | Order                                                      |
| -------------------------------------- | ---------------------------------------------------------- |
| Document                               | version marker; fields; then declarations; then ballots    |
| Ballot                                 | fields; then sections, contests, measures, and sub-ballots |
| Section                                | fields; then contests and measures                         |
| Contest                                | fields; campaigns; recommendations                         |
| Campaign, party campaign               | members; then fields                                       |
| Recommendation                         | `support` tiers; `oppose`; then `justification` and `note` |
| Measure                                | fields; recommendations                                    |
| Party, Candidate, Member, inline Party | fields only                                                |

Fields keep their source order relative to one another. Children that carry meaning in their order
(campaigns, members, tiers, items) are never reordered relative to one another.

Whether a formatter should instead place fields first in every block, or impose an order among
fields, is undecided and is expected to become a configurable option rather than a rule here.

### 6.4. Modifiers

A `Requirement` or `Extensibility` is _redundant_ when it is the value its construct would inherit
without it: the nearest modifier of the same kind written on an enclosing section or ballot, or,
when there is none, `optional` or `exhaustive`. `exhaustive` before a `retention` contest is always
redundant, since such a contest is always exhaustive. A formatter removes every redundant modifier,
along with the space after it. It leaves `extensible` before a retention contest in place; that is
an error for the author to fix.

```oval counter-example
optional exhaustive ballot @city "City of Ashford"
  required extensible section "City Offices"
    required extensible contest @mayor "Mayor" single
      ...
    optional exhaustive contest @auditor "City Auditor" single
      ...
```

A formatter rewrites the document above as the one below. The ballot's modifiers are what it would
have with nothing written, and the mayor's are what it would inherit from the section. The auditor's
stay, because without them the contest would inherit `required` and `extensible`.

```oval example
ballot @city "City of Ashford"
  required extensible section "City Offices"
    contest @mayor "Mayor" single
      ...
    optional exhaustive contest @auditor "City Auditor" single
      ...
```

Removing a redundant modifier leaves every question's effective requirement and every contest's
effective extensibility unchanged, since each construct inherits the same values it held before. The
formatted document therefore compiles to the same output, and a second pass finds nothing more to
remove. The grammar fixes the order of the two modifiers, so a formatter never reorders them.

### 6.5. References

A `support` or `oppose` reference may name any member of the campaign it resolves to. A formatter
rewrites each reference that names a member other than the campaign's head to name the head. A
reference that resolves to no campaign is left as written.

```oval counter-example
contest @governor "Governor and Lieutenant Governor" single
  campaign
    @jlee "Jordan Lee" as "Governor"
    @mpatel "Maya Patel" as "Lieutenant Governor"
  recommend
    support @mpatel
```

A formatter rewrites the recommendation above as:

```oval example
contest @governor "Governor and Lieutenant Governor" single
  campaign
    @jlee "Jordan Lee" as "Governor"
    @mpatel "Maya Patel" as "Lieutenant Governor"
  recommend
    support @jlee
```

Both name the same campaign, and the canonical output records a choice by its campaign's head, so
the rewrite does not change the output.

### 6.6. Voting methods

A formatter writes the modifiers of a contest kind in the order `seats`, `max`, `min`, with `all`
last. It removes a modifier that states its default: `seats 1` after `single` or `ranked`, `seats`
equal to the number of votes after `multiple`, and `min 1`.

```oval counter-example
contest @board "School Board" ranked min 1 seats 1 max 6
contest @council "City Council" multiple 3 min 2 seats 3
```

A formatter rewrites the headers above as:

```oval example
contest @board "School Board" ranked max 6
contest @council "City Council" multiple 3 min 2
```

Both pairs compile to the same method, so neither change alters the canonical output.

---

## A. Appendix: Grammar Summary

Every production of Section 2, collected. Lexical productions match characters; syntactic
productions match tokens. Layout tokens (`Newline`, `Indent`, `Dedent`) are produced as described in
Section 2.4.

**Lexical productions**

SourceCharacter :: "any Unicode code point except U+0000"

LineTerminator ::

- "New Line (U+000A)"
- "Carriage Return (U+000D)" "New Line (U+000A)"
- "Carriage Return (U+000D)"

Token ::

- Word
- String
- Integer
- Date
- Url
- Punctuator
- MarkdownBlock

Ignored ::

- Comment
- Whitespace
- BlankLine
- ByteOrderMark

LayoutToken ::

- Newline
- Indent
- Dedent

Newline :: "the end of a line that contains tokens"

Indent :: "an increase in indentation, at the start of a line"

Dedent :: "a return to an enclosing indentation, at the start of a line or at the end of input"

Word :: Letter WordPart*

WordPart ::

- Letter
- Digit
- `-`
- `_`

Letter :: /[A-Za-z]/

Digit :: /[0-9]/

String :: `"` StringCharacter* `"`

StringCharacter ::

- SourceCharacter but not `"` or `\` or LineTerminator
- EscapedCharacter

EscapedCharacter :: one of `\"` `\\`

Integer :: Digit+

Date :: Digit Digit Digit Digit `-` Digit Digit `-` Digit Digit

Url :: UrlScheme `://` UrlCharacter+

UrlScheme :: one of `https` `http`

UrlCharacter :: SourceCharacter but not one of "Space (U+0020)", "Tab (U+0009)", LineTerminator

Punctuator :: one of `@` `:` `,`

MarkdownBlock :: BodyLine+

BodyLine :: "a line indented more than the opening line, or a blank line between two such lines"

Comment :: `#` CommentCharacter*

CommentCharacter :: SourceCharacter but not LineTerminator

Whitespace :: "Space (U+0020)"+

BlankLine :: "a LineTerminator ending a line with no tokens but Whitespace, Comment, ByteOrderMark"

ByteOrderMark :: "Byte Order Mark (U+FEFF) at the start of the input"

**Syntactic productions**

Document : VersionMarker? GuideField* Declaration* Ballot*

VersionMarker : `oval` Integer Newline

GuideField :

- `guide` String Newline
- `election` String Newline
- `date` Date Newline
- `note` MarkdownBody

Declaration :

- Party
- Candidate

MarkdownBody : `:` Newline MarkdownBlock

StringList :

- `none`
- Strings

Strings :

- String
- Strings `,` String

Flags : Flag*

Flag : one of `incumbent` `withdrawn`

Requirement : one of `required` `optional`

Extensibility : one of `extensible` `exhaustive`

Identifier : `@` Word

Reference : Identifier

Party : `party` Identifier String Newline PartyBlock?

PartyBlock : Indent PartyField+ Dedent

PartyField :

- `abbreviation` String Newline
- `website` Url Newline
- `note` MarkdownBody

Candidate : `candidate` Identifier String Newline CandidateBlock?

CandidateBlock : Indent CandidateField+ Dedent

CandidateField :

- `designation` String Newline
- PartyRef
- `note` MarkdownBody

PartyRef :

- `party` Identifier Newline
- `party` Identifier String Newline PartyBlock?

Ballot : Requirement? Extensibility? `ballot` Identifier String Newline BallotBlock

BallotBlock : Indent BallotEntry+ Dedent

BallotEntry :

- BallotField
- BallotItem

BallotField :

- `eligibility` String Newline
- `note` MarkdownBody

BallotItem :

- Section
- Contest
- Measure
- Ballot

Section : Requirement? Extensibility? `section` String Newline SectionBlock

SectionBlock : Indent SectionEntry+ Dedent

SectionEntry :

- SectionField
- SectionItem

SectionField : `note` MarkdownBody

SectionItem :

- Contest
- Measure

Contest : Requirement? Extensibility? `contest` Identifier String ContestKind Newline ContestBlock

ContestKind :

- `single` Seats?
- `multiple` Integer MultipleModifier*
- `ranked` RankedModifier*
- `retention`

Seats : `seats` Integer

Minimum : `min` Integer

RankedMax : `max` Integer

MultipleModifier :

- Seats
- Minimum

RankedModifier :

- Seats
- RankedMax
- Minimum
- `all`

ContestBlock : Indent ContestEntry+ Dedent

ContestEntry :

- ContestField
- Campaign
- ContestRecommend

ContestField :

- `subtitle` String Newline
- `tabulation` String Newline
- `note` MarkdownBody

Campaign :

- `campaign` Flags Member Newline CampaignBlock?
- `campaign` Flags Newline CampaignBlock
- `campaign` Flags `party` PartyMember Newline PartyCampaignBlock?
- `campaign` Flags `party` Newline PartyCampaignBlock

CampaignBlock : Indent CampaignEntry+ Dedent

CampaignEntry :

- Member Newline MemberBlock?
- CampaignField

Member : Identifier String? MemberRole?

MemberRole : `as` String

MemberBlock : Indent CandidateField+ Dedent

CampaignField :

- `website` Url Newline
- `note` MarkdownBody
- `designation` String Newline
- PartyRef

PartyMember : Identifier String?

PartyCampaignBlock : Indent PartyCampaignEntry+ Dedent

PartyCampaignEntry :

- PartyMember Newline PartyBlock?
- PartyCampaignField

PartyCampaignField :

- `website` Url Newline
- `note` MarkdownBody

ContestRecommend :

- `recommend` Newline StanceBlock
- `recommend` NoStance Newline RecommendFieldBlock?

NoStance : one of `no-position` `abstain`

StanceBlock : Indent RecommendField* Stance RecommendEntry* Dedent

RecommendEntry :

- Stance
- RecommendField

Stance :

- Support
- Oppose

Support : `support` Choice+ Newline

Oppose : `oppose` Choice+ Newline

Choice :

- Reference
- WriteIn

WriteIn : `write-in` String?

RecommendField :

- `justification` MarkdownBody
- `note` MarkdownBody

RecommendFieldBlock : Indent RecommendField+ Dedent

Measure : Requirement? `measure` Identifier String Newline MeasureBlock

MeasureBlock : Indent MeasureEntry+ Dedent

MeasureEntry :

- MeasureField
- MeasureRecommend

MeasureField :

- `subtitle` String Newline
- `description` MarkdownBody
- `if_yes` MarkdownBody
- `if_no` MarkdownBody
- `placement` String Newline
- `tabulation` String Newline
- `fiscal_impact` String Newline
- `supporters` StringList Newline
- `opponents` StringList Newline
- `note` MarkdownBody

MeasureRecommend : `recommend` MeasurePosition Newline RecommendFieldBlock?

MeasurePosition : one of `yes` `no` `no-position` `abstain`

## B. Appendix: Keywords

A keyword is a `Word` that a syntactic production in Section 2 names literally. This list is derived
from the grammar; nothing is reserved, since identifiers begin with `@`.

| Group            | Keywords                                                                                         |
| ---------------- | ------------------------------------------------------------------------------------------------ |
| Document         | `oval` `guide` `election` `date` `note`                                                          |
| Entities         | `party` `candidate` `ballot` `section` `contest` `campaign` `measure`                            |
| Party fields     | `abbreviation` `website`                                                                         |
| Candidate fields | `designation`                                                                                    |
| Ballot fields    | `eligibility`                                                                                    |
| Contest header   | `single` `multiple` `ranked` `retention` `seats` `max` `min` `all`                               |
| Contest fields   | `subtitle` `tabulation`                                                                          |
| Campaign         | `incumbent` `withdrawn` `as`                                                                     |
| Requirement      | `required` `optional`                                                                            |
| Extensibility    | `extensible` `exhaustive`                                                                        |
| Recommendation   | `recommend` `support` `oppose` `justification` `yes` `no` `no-position` `abstain` `write-in`     |
| Measure fields   | `description` `if_yes` `if_no` `placement` `tabulation` `fiscal_impact` `supporters` `opponents` |
| Values           | `none`                                                                                           |

## C. Copyright and Licensing

**Copyright notice**

Copyright © 2026 Oval contributors.

This specification, including its grammar, schema, and examples, is licensed under the
[Creative Commons Attribution 4.0 International License](https://creativecommons.org/licenses/by/4.0/).

The source code of the reference implementation is licensed under the
[MIT License](https://opensource.org/licenses/MIT).
