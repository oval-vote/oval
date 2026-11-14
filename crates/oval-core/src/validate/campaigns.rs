use oval_syntax::ast::{AstNode, Campaign, Document};

use super::{content_range, diagnostic, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn misplaced_candidate_fields(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for campaign in kept_nodes(document.syntax()).filter_map(Campaign::cast) {
        let describes_a_candidate = !campaign.is_block_form()
            && campaign
                .members()
                .next()
                .is_some_and(|member| member.is_declaration());
        if describes_a_candidate {
            continue;
        }
        let designation = campaign.designation().map(|field| field.syntax().clone());
        let party = campaign.party().map(|field| field.syntax().clone());
        for field in designation.into_iter().chain(party) {
            if let Some(range) = content_range(&field) {
                diagnostics.push(diagnostic(ValidationRule::MisplacedCandidateField, range));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::reported_as;

    #[test]
    fn a_candidate_field_at_a_campaigns_level_that_describes_no_candidate_is_misplaced() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\ncandidate @a \"A\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a\n      designation \"S\"\n      party @dem\n    campaign\n      @c \"C\"\n      designation \"Q\"\n    campaign @d \"D\"\n      designation \"OK\"\n      party @dem\n    recommend no-position\n";
        assert_eq!(
            reported_as(text, "misplaced-candidate-field"),
            [(140, 155), (162, 172), (205, 220)]
        );
    }
}
