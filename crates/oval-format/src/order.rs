use oval_syntax::SyntaxKind;

use crate::entry::{Entry, Trivia};

pub(crate) fn reorder(owner: SyntaxKind, entries: Vec<Entry>) -> Vec<Entry> {
    let mut indexed: Vec<(usize, Entry)> = entries.into_iter().enumerate().collect();
    indexed.sort_by_key(|(_, entry)| group(owner, entry.kind));
    let moved_to_start = indexed.first().is_some_and(|(index, _)| *index != 0);
    let mut entries: Vec<Entry> = indexed.into_iter().map(|(_, entry)| entry).collect();
    if moved_to_start
        && let Some(first) = entries.first_mut()
        && first.leading.first() == Some(&Trivia::Blank)
    {
        first.leading.remove(0);
    }
    entries
}

fn group(owner: SyntaxKind, child: SyntaxKind) -> u8 {
    match (owner, child) {
        (SyntaxKind::Document, SyntaxKind::VersionMarker)
        | (SyntaxKind::Campaign, SyntaxKind::Member | SyntaxKind::PartyMember)
        | (SyntaxKind::ContestRecommend, SyntaxKind::Support) => 0,
        (SyntaxKind::ContestRecommend, SyntaxKind::Oppose) => 1,
        (SyntaxKind::Document, SyntaxKind::Party | SyntaxKind::Candidate)
        | (SyntaxKind::Contest, SyntaxKind::Campaign) => 3,
        (SyntaxKind::Document, SyntaxKind::Ballot)
        | (
            SyntaxKind::Ballot | SyntaxKind::Section,
            SyntaxKind::Section | SyntaxKind::Contest | SyntaxKind::Measure | SyntaxKind::Ballot,
        )
        | (SyntaxKind::Contest, SyntaxKind::ContestRecommend)
        | (SyntaxKind::Measure, SyntaxKind::MeasureRecommend) => 4,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn formatted(text: &str) -> String {
        format(text).unwrap()
    }

    #[test]
    fn a_document_keeps_its_fields_and_declarations_in_source_order() {
        let text = "oval 1\ndate 2026-11-03\nguide \"G\"\nelection \"E\"\ncandidate @c \"C\"\nparty @p \"P\"\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n";
        assert_eq!(formatted(text), text);
    }

    #[test]
    fn ballots_and_sections_have_their_fields_then_their_items_in_ballot_order() {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n  eligibility \"All\"\n  section \"S\"\n    measure @n \"N\"\n      recommend yes\n    note:\n      Section.\n    contest @k \"K\" single\n      campaign @a \"A\"\n      recommend no-position\n  ballot @c \"C\"\n    measure @o \"O\"\n      recommend no\n  note:\n    Ballot.\n"
            ),
            "oval 1\nballot @b \"B\"\n  eligibility \"All\"\n  note:\n    Ballot.\n  measure @m \"M\"\n    recommend yes\n  section \"S\"\n    note:\n      Section.\n    measure @n \"N\"\n      recommend yes\n    contest @k \"K\" single\n      campaign @a \"A\"\n      recommend no-position\n  ballot @c \"C\"\n    measure @o \"O\"\n      recommend no\n"
        );
    }

    #[test]
    fn a_contest_has_its_fields_then_campaigns_then_recommendation() {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n  contest @k \"K\" ranked\n    recommend no-position\n    campaign @z \"Z\"\n    subtitle \"Sub\"\n    campaign @a \"A\"\n    tabulation \"IRV\"\n"
            ),
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" ranked\n    subtitle \"Sub\"\n    tabulation \"IRV\"\n    campaign @z \"Z\"\n    campaign @a \"A\"\n    recommend no-position\n"
        );
    }

    #[test]
    fn a_campaign_has_its_members_then_fields() {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign\n      website https://x.example\n      @z \"Z\"\n        note:\n          Z.\n        designation \"D\"\n      note:\n        Ticket.\n      @a \"A\"\n    campaign party\n      website https://p.example\n      @p \"P\"\n    recommend no-position\n"
            ),
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign\n      @z \"Z\"\n        note:\n          Z.\n        designation \"D\"\n      @a \"A\"\n      website https://x.example\n      note:\n        Ticket.\n    campaign party\n      @p \"P\"\n      website https://p.example\n    recommend no-position\n"
        );
    }

    #[test]
    fn a_recommendation_has_its_tiers_then_oppose_then_fields() {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n  contest @k \"K\" ranked\n    campaign @a \"A\"\n    campaign @c \"C\"\n    campaign @d \"D\"\n    recommend\n      note:\n        Note.\n      oppose @d\n      support @c\n      justification:\n        Why.\n      support @a\n  measure @m \"M\"\n    recommend yes\n    note:\n      Measure.\n"
            ),
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" ranked\n    campaign @a \"A\"\n    campaign @c \"C\"\n    campaign @d \"D\"\n    recommend\n      support @c\n      support @a\n      oppose @d\n      note:\n        Note.\n      justification:\n        Why.\n  measure @m \"M\"\n    note:\n      Measure.\n    recommend yes\n"
        );
    }

    #[test]
    fn comments_move_with_their_entry_and_a_moved_entry_loses_a_blank_line_that_would_begin_a_block()
     {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n\n  # about the measure\n  measure @m \"M\"\n    recommend yes\n\n  # about the note\n  note:\n    Note.\n  contest @k \"K\" single\n\n    # about the recommendation\n    recommend no-position\n\n    # about the campaign\n    campaign @a \"A\"\n"
            ),
            "oval 1\nballot @b \"B\"\n  # about the note\n  note:\n    Note.\n\n  # about the measure\n  measure @m \"M\"\n    recommend yes\n  contest @k \"K\" single\n    # about the campaign\n    campaign @a \"A\"\n\n    # about the recommendation\n    recommend no-position\n"
        );
    }

    #[test]
    fn a_blank_line_at_the_start_of_a_block_stays_when_its_entry_stays_first() {
        let text = "oval 1\nballot @b \"B\"\n\n  note:\n    Note.\n  measure @m \"M\"\n    recommend yes\n";
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n\n  note:\n    Note.\n  measure @m \"M\"\n    recommend yes\n"
            ),
            text
        );
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n\n  note:\n    Note.\n  measure @m \"M\"\n    recommend yes\n  eligibility \"All\"\n"
            ),
            "oval 1\nballot @b \"B\"\n\n  note:\n    Note.\n  eligibility \"All\"\n  measure @m \"M\"\n    recommend yes\n"
        );
    }
}
