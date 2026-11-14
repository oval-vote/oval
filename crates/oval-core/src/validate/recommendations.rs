use std::collections::HashSet;

use oval_syntax::ast::{
    AstNode, AstToken, Choice, Contest, ContestKind, ContestRecommend, Document, Measure, Member,
    Method, Stance, Support,
};
use oval_syntax::{SyntaxNode, TextRange};

use super::campaigns::members;
use super::index::{Entity, Index};
use super::{content_range, diagnostic, header_range, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn recommendation_counts(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for node in kept_nodes(document.syntax()) {
        let recommendations: Vec<SyntaxNode> = if let Some(contest) = Contest::cast(node.clone()) {
            contest
                .recommendations()
                .map(|recommendation| recommendation.syntax().clone())
                .collect()
        } else if let Some(measure) = Measure::cast(node.clone()) {
            measure
                .recommendations()
                .map(|recommendation| recommendation.syntax().clone())
                .collect()
        } else {
            continue;
        };
        if recommendations.is_empty() {
            push(
                ValidationRule::MissingRecommendation,
                header_range(&node),
                diagnostics,
            );
        }
        for repeated in recommendations.iter().skip(1) {
            push(
                ValidationRule::RepeatedRecommendation,
                content_range(repeated),
                diagnostics,
            );
        }
    }
}

pub(super) fn stances(document: &Document, index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        let campaigns: Vec<HashSet<String>> = contest
            .campaigns()
            .map(|campaign| {
                members(&campaign)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect()
            })
            .collect();
        let kind = contest.kind();
        let method = kind.as_ref().map(ContestKind::method);
        let candidates: HashSet<String> = contest
            .campaigns()
            .flat_map(|campaign| campaign.members().collect::<Vec<_>>())
            .filter_map(|member| candidate_name(&member, index))
            .collect();
        for recommendation in contest.recommendations() {
            let stances = Stances {
                campaigns: &campaigns,
                index,
                recommendation: &recommendation,
            };
            stances.repeated_lines(method, diagnostics);
            stances.repeated_choices(diagnostics);
            stances.campaigns_not_in_contest(diagnostics);
            if method == Some(Method::Retention) {
                stances.retention_shape(diagnostics);
            }
            if let Some(kind) = &kind {
                stances.support_counts(kind, diagnostics);
            }
            stances.write_ins_on_ballot(&candidates, diagnostics);
        }
    }
}

struct Stances<'a> {
    campaigns: &'a [HashSet<String>],
    index: &'a Index,
    recommendation: &'a ContestRecommend,
}

impl Stances<'_> {
    fn repeated_lines(&self, method: Option<Method>, diagnostics: &mut Vec<Diagnostic>) {
        for oppose in self.recommendation.opposes().skip(1) {
            push(
                ValidationRule::RepeatedOppose,
                header_range(oppose.syntax()),
                diagnostics,
            );
        }
        if method == Some(Method::Single) {
            for support in self.recommendation.supports().skip(1) {
                push(
                    ValidationRule::TiersInSingle,
                    header_range(support.syntax()),
                    diagnostics,
                );
            }
        }
    }

    fn repeated_choices(&self, diagnostics: &mut Vec<Diagnostic>) {
        let mut identifiers = HashSet::new();
        let mut campaigns = HashSet::new();
        let mut write_ins = HashSet::new();
        for choice in self.choices() {
            let repeated = match &choice {
                Choice::Reference(identifier) => {
                    let Some(name) = identifier.name().map(|name| name.text().to_owned()) else {
                        continue;
                    };
                    let campaign = self.campaign_of(&name);
                    let repeated = identifiers.contains(&name)
                        || campaign.is_some_and(|campaign| campaigns.contains(&campaign));
                    identifiers.insert(name);
                    campaigns.extend(campaign);
                    repeated
                }
                Choice::WriteIn(write_in) => {
                    let name = write_in.name().map(|name| name.value().into_owned());
                    !write_ins.insert(name)
                }
            };
            if repeated {
                push(
                    ValidationRule::RepeatedStance,
                    Some(choice.syntax().text_range()),
                    diagnostics,
                );
            }
        }
    }

    fn campaigns_not_in_contest(&self, diagnostics: &mut Vec<Diagnostic>) {
        for choice in self.choices() {
            let Choice::Reference(identifier) = choice else {
                continue;
            };
            let (Some(name), Some(declaration)) =
                (identifier.name(), self.index.resolve(&identifier))
            else {
                continue;
            };
            let entity = self.index.declarations[declaration].entity;
            let names_a_campaign = matches!(entity, Entity::Candidate | Entity::Party);
            if names_a_campaign && self.campaign_of(name.text()).is_none() {
                push(
                    ValidationRule::CampaignNotInContest,
                    Some(identifier.syntax().text_range()),
                    diagnostics,
                );
            }
        }
    }

    fn retention_shape(&self, diagnostics: &mut Vec<Diagnostic>) {
        for (position, stance) in self.recommendation.stances().enumerate() {
            let names_only_the_campaign = stance.choices().all(|choice| match choice {
                Choice::Reference(identifier) => identifier
                    .name()
                    .is_some_and(|name| self.campaign_of(name.text()) == Some(0)),
                Choice::WriteIn(_) => false,
            });
            if position > 0 || !names_only_the_campaign {
                push(
                    ValidationRule::RetentionStance,
                    header_range(stance.syntax()),
                    diagnostics,
                );
            }
        }
    }

    fn support_counts(&self, kind: &ContestKind, diagnostics: &mut Vec<Diagnostic>) {
        let (limit, rule) = match kind.method() {
            Method::Ranked => (kind.max(), ValidationRule::SupportOverMax),
            Method::Multiple => (kind.votes(), ValidationRule::SupportOverCount),
            Method::Single | Method::Retention => return,
        };
        let supports: Vec<Support> = self.recommendation.supports().collect();
        let choices: Vec<Choice> = supports.iter().flat_map(Support::choices).collect();
        let beyond = limit
            .and_then(|limit| usize::try_from(limit.value()?).ok())
            .and_then(|limit| choices.get(limit));
        if let Some(beyond) = beyond {
            push(rule, Some(beyond.syntax().text_range()), diagnostics);
        }
        let Some(last) = supports.last() else {
            return;
        };
        let below_min = kind.min().is_some_and(|min| {
            min.value()
                .and_then(|min| usize::try_from(min).ok())
                .is_none_or(|min| choices.len() < min)
        });
        let below_all = kind.method() == Method::Ranked
            && kind.all().is_some()
            && choices.len() < self.campaigns.len();
        if below_min || below_all {
            push(
                ValidationRule::SupportBelowMinimum,
                header_range(last.syntax()),
                diagnostics,
            );
        }
    }

    fn write_ins_on_ballot(&self, candidates: &HashSet<String>, diagnostics: &mut Vec<Diagnostic>) {
        for choice in self.choices() {
            if let Choice::WriteIn(write_in) = choice
                && let Some(name) = write_in.name()
                && candidates.contains(name.value().as_ref())
            {
                push(
                    ValidationRule::WriteInOnBallot,
                    Some(name.syntax().text_range()),
                    diagnostics,
                );
            }
        }
    }

    fn choices(&self) -> impl Iterator<Item = Choice> + use<> {
        self.recommendation
            .stances()
            .flat_map(|stance: Stance| stance.choices().collect::<Vec<_>>())
    }

    fn campaign_of(&self, name: &str) -> Option<usize> {
        self.campaigns
            .iter()
            .position(|members| members.contains(name))
    }
}

fn candidate_name(member: &Member, index: &Index) -> Option<String> {
    let declaration = &index.declarations[index.resolve(&member.identifier()?)?];
    if declaration.entity != Entity::Candidate {
        return None;
    }
    Some(declaration.name()?.value().into_owned())
}

fn push(rule: ValidationRule, range: Option<TextRange>, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(range) = range {
        diagnostics.push(diagnostic(rule, range));
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    #[test]
    fn every_contest_and_measure_has_exactly_one_recommendation() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  required contest @k \"K\" single  # c\n    campaign @a \"A\"\n  measure @m \"M\"\n    subtitle \"S\"\n  contest @k2 \"K2\" single\n    campaign @a2 \"A2\"\n    recommend no-position\n    recommend\n      support @a2\n      note:\n        N.\n    recommend abstain\n  measure @m2 \"M2\"\n    recommend yes\n    recommend no\n  measure @m3 \"M3\"\n    bogus recommend\n";
        assert_eq!(
            reported(text),
            [
                ("missing-recommendation", 62, 92),
                ("missing-recommendation", 120, 134),
                ("repeated-recommendation", 230, 281),
                ("repeated-recommendation", 285, 302),
                ("repeated-recommendation", 344, 356),
                ("missing-recommendation", 359, 375),
                ("unexpected-entry", 380, 395),
            ]
        );
    }

    const STANCES: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\ncandidate @n \"N\"\n  party @dem\nballot @b \"B\"\n  extensible contest @k \"K\" ranked\n    campaign\n      @a \"A\"\n      @a2 \"A2\"\n    campaign @c \"C\"\n    campaign party @g \"G\"\n    recommend\n      support @a write-in \"J\" write-in\n      support @a2 @ghost @ghost\n      oppose @c @n write-in \"J\"\n      oppose write-in @dem @g\n";

    #[test]
    fn a_choice_repeating_an_identifier_campaign_or_write_in_is_a_repeated_stance() {
        assert_eq!(
            reported_as(STANCES, "repeated-stance"),
            [(296, 299), (307, 313), (333, 345), (359, 367)]
        );
    }

    #[test]
    fn a_reference_to_a_candidate_or_party_outside_the_contests_campaigns_is_reported() {
        assert_eq!(
            reported_as(STANCES, "campaign-not-in-contest"),
            [(330, 332), (368, 372)]
        );
        assert_eq!(
            reported_as(STANCES, "undeclared-reference"),
            [(300, 306), (307, 313)]
        );
    }

    #[test]
    fn a_reference_to_another_kind_of_declaration_is_mismatched_rather_than_not_in_contest() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @s \"S\" single\n    campaign @x \"X\"\n    recommend\n      support @s\n";
        assert_eq!(reported(text), [("mismatched-reference", 132, 134)]);
    }

    #[test]
    fn each_oppose_line_after_the_first_is_repeated() {
        assert_eq!(reported_as(STANCES, "repeated-oppose"), [(352, 375)]);
    }

    #[test]
    fn a_single_contest_has_one_tier() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @s \"S\" single\n    campaign @x \"X\"\n    campaign @y \"Y\"\n    recommend\n      support @x\n      support @y\n      support @y\n";
        assert_eq!(
            reported(text),
            [
                ("tiers-in-single", 161, 171),
                ("tiers-in-single", 178, 188),
                ("repeated-stance", 186, 188),
            ]
        );
        assert_eq!(reported_as(STANCES, "tiers-in-single"), []);
    }

    #[test]
    fn a_retention_recommendation_is_one_line_naming_only_the_first_campaign() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @r1 \"R1\" retention\n    campaign @u \"U\"\n    recommend\n      support @u\n      oppose @u\n  contest @r2 \"R2\" retention\n    campaign @v \"V\"\n    recommend\n      oppose @v @v\n  contest @r3 \"R3\" retention\n    campaign @w \"W\"\n    campaign @w2 \"W2\"\n    recommend\n      support @w2\n  contest @r4 \"R4\" retention\n    campaign @z \"Z\"\n    recommend\n      justification:\n        J.\n      support write-in \"Z\"\n  contest @r5 \"R5\" retention\n    campaign @q \"Q\"\n    recommend no-position\n";
        assert_eq!(
            reported_as(text, "retention-stance"),
            [(146, 155), (329, 340), (442, 462)]
        );
    }

    #[test]
    fn a_write_in_matching_a_party_named_by_a_mismatched_member_is_not_on_the_ballot() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\nextensible ballot @b \"B\"\n  contest @k \"K\" single\n    campaign @dem\n    recommend\n      support write-in \"Dem\"\n";
        assert_eq!(reported_as(text, "mismatched-reference").len(), 1);
        assert_eq!(reported_as(text, "write-in-on-ballot"), []);
    }

    #[test]
    fn support_is_counted_against_max_votes_min_and_every_campaign_for_all() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @r \"R\" ranked max 2\n    campaign @a \"A\"\n    campaign @c \"C\"\n    campaign @d \"D\"\n    recommend\n      support @a\n      support @c @d\n  contest @m \"M\" multiple 2 seats 3\n    campaign @a2 \"A\"\n    campaign @c2 \"C\"\n    campaign @d2 \"D\"\n    recommend\n      support @a2 @c2\n      support @d2\n  contest @n \"N\" ranked min 3\n    campaign @a3 \"A\"\n    campaign @c3 \"C\"\n    campaign @d3 \"D\"\n    recommend\n      support @a3\n      support @c3\n  contest @l \"L\" ranked all\n    campaign @a4 \"A\"\n    campaign @c4 \"C\"\n    campaign @d4 \"D\"\n    recommend\n      support @a4 @c4\n  contest @o \"O\" multiple 2 min 2\n    campaign @a5 \"A\"\n    campaign @c5 \"C\"\n    recommend\n      support @a5 @c5\n  contest @h \"H\" ranked max 99999999999999999999 min 99999999999999999999\n    campaign @a6 \"A\"\n    recommend\n      support @a6\n";
        assert_eq!(
            reported(text),
            [
                ("support-over-max", 198, 200),
                ("support-over-count", 350, 353),
                ("support-below-minimum", 485, 496),
                ("support-below-minimum", 608, 623),
                ("support-below-minimum", 851, 862),
            ]
        );
    }
}
