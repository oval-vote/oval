use oval_syntax::SyntaxKind;
use oval_syntax::ast::{
    AstNode, AstToken, Campaign, Contest, ContestKind, Document, IntegerLiteral, Method,
};

use super::{content_range, diagnostic, header_range, kept_nodes};
use crate::{Diagnostic, ValidationRule};

const LARGEST_INTEGER: u64 = 9_007_199_254_740_991;

pub(super) fn campaign_counts(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        let Some(kind) = contest.kind() else {
            continue;
        };
        let campaigns: Vec<Campaign> = contest.campaigns().collect();
        let header = header_range(contest.syntax()).expect("a contest begins with its keyword");
        if kind.method() != Method::Retention {
            if campaigns.is_empty() {
                diagnostics.push(diagnostic(ValidationRule::ContestWithoutCampaigns, header));
            }
            continue;
        }
        if campaigns.is_empty() {
            diagnostics.push(diagnostic(ValidationRule::RetentionCampaignCount, header));
        }
        for extra in campaigns.iter().skip(1) {
            if let Some(range) = content_range(extra.syntax()) {
                diagnostics.push(diagnostic(ValidationRule::RetentionCampaignCount, range));
            }
        }
        for campaign in campaigns.iter().filter(|campaign| campaign.is_party()) {
            if let Some(range) = header_range(campaign.syntax()) {
                diagnostics.push(diagnostic(ValidationRule::RetentionPartyCampaign, range));
            }
        }
    }
}

pub(super) fn contest_kinds(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for kind in kept_nodes(document.syntax()).filter_map(ContestKind::cast) {
        let mut report = |rule: ValidationRule, integer: &IntegerLiteral| {
            diagnostics.push(diagnostic(rule, integer.syntax().text_range()));
        };
        if let Some(votes) = kind.votes()
            && votes.value().is_some_and(|votes| votes < 2)
        {
            report(ValidationRule::InvalidMultipleCount, &votes);
        }
        if let Some(max) = kind.max()
            && max.value() == Some(0)
        {
            report(ValidationRule::InvalidRankedMax, &max);
        }
        if let Some(seats) = kind.seats()
            && seats.value() == Some(0)
        {
            report(ValidationRule::InvalidSeatCount, &seats);
        }
        if let Some(min) = kind.min() {
            let limit = match kind.method() {
                Method::Ranked => kind.max(),
                Method::Multiple => kind.votes(),
                Method::Single | Method::Retention => None,
            };
            if min.value() == Some(0) || limit.is_some_and(|limit| exceeds(&min, &limit)) {
                report(ValidationRule::InvalidMinimum, &min);
            }
        }
        let integers = [kind.votes(), kind.seats(), kind.max(), kind.min()];
        for integer in integers.into_iter().flatten() {
            if integer.value().is_none_or(|value| value > LARGEST_INTEGER) {
                report(ValidationRule::IntegerTooLarge, &integer);
            }
        }
        let has_limit = kind
            .syntax()
            .children()
            .any(|modifier| matches!(modifier.kind(), SyntaxKind::RankedMax | SyntaxKind::Minimum));
        if let Some(all) = kind.all()
            && has_limit
        {
            diagnostics.push(diagnostic(
                ValidationRule::RankedAllWithLimit,
                all.text_range(),
            ));
        }
    }
}

fn exceeds(integer: &IntegerLiteral, limit: &IntegerLiteral) -> bool {
    magnitude(integer) > magnitude(limit)
}

fn magnitude(integer: &IntegerLiteral) -> (usize, &str) {
    let digits = integer.text().trim_start_matches('0');
    (digits.len(), digits)
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    const CAMPAIGNS: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  required contest @a \"A\" single  # c\n    subtitle \"S\"\n  contest @k \"K\"\n    subtitle \"S\"\n  contest @u \"U\" single\n    bogus 1\n  contest @r \"R\" retention\n    subtitle \"S\"\n  contest @s \"S\" retention\n    campaign @x \"X\"\n    campaign party @dem \"Dem\"\n      website https://dem.example\n    campaign party\n      @g \"G\"\n  contest @t \"T\" retention\n    campaign party @h \"H\"\n";

    fn kind_errors(kind: &str) -> Vec<(&'static str, u32, u32)> {
        let text = format!(
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @c \"C\" {kind}\n    campaign @x \"X\"\n    recommend no-position\n"
        );
        let start = u32::try_from(text.find(kind).unwrap()).unwrap();
        reported(&text)
            .into_iter()
            .map(|(id, from, to)| (id, from - start, to - start))
            .collect()
    }

    #[test]
    fn a_contest_of_a_kind_other_than_retention_needs_a_campaign() {
        assert_eq!(
            reported_as(CAMPAIGNS, "contest-without-campaigns"),
            [(62, 92), (151, 172)]
        );
    }

    #[test]
    fn a_retention_contest_needs_exactly_one_campaign() {
        assert_eq!(
            reported_as(CAMPAIGNS, "retention-campaign-count"),
            [(187, 211), (280, 339), (344, 371)]
        );
    }

    #[test]
    fn each_party_campaign_in_a_retention_contest_is_invalid() {
        assert_eq!(
            reported_as(CAMPAIGNS, "retention-party-campaign"),
            [(280, 305), (344, 358), (403, 424)]
        );
    }

    #[test]
    fn a_multiple_contest_gives_at_least_two_votes() {
        assert_eq!(kind_errors("multiple 2"), []);
        assert_eq!(
            kind_errors("multiple 0001"),
            [("invalid-multiple-count", 9, 13)]
        );
    }

    #[test]
    fn max_and_seats_are_at_least_one() {
        assert_eq!(kind_errors("single seats 1"), []);
        assert_eq!(
            kind_errors("ranked max 0 seats 00 min 1"),
            [
                ("invalid-ranked-max", 11, 12),
                ("invalid-seat-count", 19, 21),
                ("invalid-minimum", 26, 27),
            ]
        );
    }

    #[test]
    fn min_is_at_least_one_and_at_most_the_max_or_the_number_of_votes() {
        assert_eq!(kind_errors("ranked max 3 min 3"), []);
        assert_eq!(
            kind_errors("ranked max 3 min 4"),
            [("invalid-minimum", 17, 18)]
        );
        assert_eq!(
            kind_errors("multiple 3 min 0"),
            [("invalid-minimum", 15, 16)]
        );
        assert_eq!(
            kind_errors("multiple 1 min 2"),
            [
                ("invalid-multiple-count", 9, 10),
                ("invalid-minimum", 15, 16)
            ]
        );
    }

    #[test]
    fn numbers_too_large_for_a_u64_are_compared_exactly() {
        assert_eq!(
            kind_errors("multiple 99999999999999999999 min 0099999999999999999998"),
            [("integer-too-large", 9, 29), ("integer-too-large", 34, 56),]
        );
        assert_eq!(
            kind_errors("ranked max 99999999999999999999 min 100000000000000000000"),
            [
                ("integer-too-large", 11, 31),
                ("invalid-minimum", 36, 57),
                ("integer-too-large", 36, 57),
            ]
        );
    }

    #[test]
    fn a_number_past_the_largest_integer_json_reads_exactly_is_too_large() {
        assert_eq!(
            kind_errors("multiple 9007199254740991 seats 0009007199254740991"),
            []
        );
        assert_eq!(
            kind_errors("ranked seats 9007199254740992 max 18446744073709551616 min 2"),
            [("integer-too-large", 13, 29), ("integer-too-large", 34, 54),]
        );
        assert_eq!(
            kind_errors("multiple 9007199254740992 min 9007199254740993"),
            [
                ("integer-too-large", 9, 25),
                ("invalid-minimum", 30, 46),
                ("integer-too-large", 30, 46),
            ]
        );
    }

    #[test]
    fn all_leaves_no_room_for_max_or_min() {
        assert_eq!(kind_errors("ranked all seats 2"), []);
        assert_eq!(
            kind_errors("ranked all max 2"),
            [("ranked-all-with-limit", 7, 10)]
        );
        assert_eq!(
            kind_errors("ranked all min"),
            [("ranked-all-with-limit", 7, 10), ("missing-value", 14, 14)]
        );
    }
}
