use oval_syntax::ast::{
    AstNode, Choice, Contest, ContestRecommend, Document, Extensibility, MeasureRecommend, Method,
    NoStance, Position, Requirement,
};
use oval_syntax::{SyntaxKind, SyntaxNode};

use super::{diagnostic, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn abstentions_on_required(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for node in kept_nodes(document.syntax()) {
        let abstain = if let Some(recommendation) = ContestRecommend::cast(node.clone()) {
            recommendation
                .no_stance()
                .filter(NoStance::is_abstain)
                .map(|no_stance| no_stance.syntax().clone())
        } else if let Some(recommendation) = MeasureRecommend::cast(node.clone()) {
            recommendation
                .position()
                .filter(|position| position.value() == Position::Abstain)
                .map(|position| position.syntax().clone())
        } else {
            None
        };
        let Some(abstain) = abstain else {
            continue;
        };
        if nearest::<Requirement>(&node).is_some_and(|requirement| requirement.is_required()) {
            diagnostics.push(diagnostic(
                ValidationRule::AbstainOnRequired,
                abstain.text_range(),
            ));
        }
    }
}

pub(super) fn write_ins_not_accepted(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        let extensible = !is_retention(&contest)
            && nearest::<Extensibility>(contest.syntax())
                .is_some_and(|extensibility| extensibility.is_extensible());
        if extensible {
            continue;
        }
        let choices = contest
            .recommendations()
            .flat_map(|recommendation| recommendation.stances().collect::<Vec<_>>())
            .flat_map(|stance| stance.choices().collect::<Vec<_>>());
        for choice in choices {
            if let Choice::WriteIn(write_in) = choice {
                diagnostics.push(diagnostic(
                    ValidationRule::WriteInNotAccepted,
                    write_in.syntax().text_range(),
                ));
            }
        }
    }
}

pub(super) fn extensible_retentions(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        if is_retention(&contest)
            && let Some(extensibility) = contest.extensibility()
            && extensibility.is_extensible()
        {
            diagnostics.push(diagnostic(
                ValidationRule::ExtensibleRetention,
                extensibility.syntax().text_range(),
            ));
        }
    }
}

fn is_retention(contest: &Contest) -> bool {
    contest
        .kind()
        .is_some_and(|kind| kind.method() == Method::Retention)
}

pub(crate) fn nearest<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.ancestors()
        .filter(|ancestor| {
            matches!(
                ancestor.kind(),
                SyntaxKind::Contest
                    | SyntaxKind::Measure
                    | SyntaxKind::Section
                    | SyntaxKind::Ballot
            )
        })
        .find_map(|ancestor| ancestor.children().find_map(N::cast))
}

#[cfg(test)]
mod tests {
    use crate::validate::reported;

    #[test]
    fn abstain_is_invalid_on_a_question_whose_nearest_requirement_is_required() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nrequired ballot @b \"B\"\n  section \"S\"\n    contest @k \"K\" single\n      campaign @a \"A\"\n      recommend abstain\n    optional measure @m \"M\"\n      recommend abstain\n  optional section \"T\"\n    measure @m2 \"M2\"\n      recommend abstain\n    required measure @m3 \"M3\"\n      recommend abstain\n  ballot @sub \"Sub\"\n    measure @m4 \"M4\"\n      recommend abstain\n";
        assert_eq!(
            reported(text),
            [
                ("abstain-on-required", 147, 154),
                ("abstain-on-required", 321, 328),
                ("abstain-on-required", 386, 393),
            ]
        );
    }

    #[test]
    fn only_an_extensible_contest_accepts_write_ins_and_retention_is_never_extensible() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\ncandidate @n \"Nora\"\nextensible ballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a \"A\"\n    recommend\n      support write-in \"Jane\"\n  exhaustive section \"S\"\n    contest @k2 \"K2\" single\n      campaign @a2 \"A2\"\n      recommend\n        support write-in\n    extensible contest @k3 \"K3\" single\n      campaign @a3 \"Ana\"\n      campaign @n\n      campaign party @g \"Green\"\n      recommend\n        support write-in \"Ana\" write-in \"Nora\" write-in \"Green\"\n  contest @r \"R\" retention\n    campaign @u \"U\"\n    recommend\n      support write-in \"X\"\n  extensible contest @r2 \"R2\" retention\n    campaign @v \"V\"\n    recommend no-position\nballot @c \"C\"\n  contest @k4 \"K4\" single\n    campaign @a4 \"A4\"\n    recommend\n      oppose write-in \"Z\"\n";
        assert_eq!(
            reported(text),
            [
                ("write-in-not-accepted", 288, 296),
                ("write-in-on-ballot", 452, 457),
                ("write-in-on-ballot", 467, 473),
                ("retention-stance", 558, 578),
                ("write-in-not-accepted", 566, 578),
                ("extensible-retention", 581, 591),
                ("write-in-not-accepted", 754, 766),
            ]
        );
    }
}
