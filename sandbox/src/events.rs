use uuid::Uuid;
use strum::Display;
use std::string::ToString;

use crate::{bases::Baserunners, entities::{Player, World}, mods::{Mod, ModLifetime}, Game, Weather};

#[derive(Display, Debug, Clone)]
pub enum Event {
    BatterUp {
        batter: Uuid
    },
    InningSwitch {
        inning: i16,
        top: bool,
    },
    GameOver,

    Ball,
    Strike,
    Foul,

    Strikeout,
    Walk,
    HomeRun,

    // todo: find a nicer way to encode runner advancement
    BaseHit {
        bases: u8,
        runners_after: Baserunners,
    },
    GroundOut {
        fielder: Uuid,
        runners_after: Baserunners,
    },
    Flyout {
        fielder: Uuid,
        runners_after: Baserunners,
    },
    DoublePlay {
        runners_after: Baserunners
    },
    FieldersChoice {
        runners_after: Baserunners
    },

    BaseSteal {
        runner: Uuid,
        base_from: u8,
        base_to: u8,
    },
    CaughtStealing {
        runner: Uuid,
        base_from: u8,
    },
    Party {
        target: Uuid,
        boosts: Vec<f64>
    },
    Incineration {
        target: Uuid,
        replacement: Player,
        chain: Option<Uuid>
    },
    Peanut {
        target: Uuid,
        yummy: bool
    },
    Birds,
    Feedback {
        target1: Uuid,
        target2: Uuid,
    },
    Reverb {
        reverb_type: u8,
        team: Uuid,
        changes: Vec<usize>
    },
    Blooddrain {
        drainer: Uuid,
        target: Uuid,
        stat: u8,
        siphon: bool,
        siphon_effect: i16
    },
    Sun2 {
        home_team: bool,
    },
    BlackHole {
        home_team: bool,
    },
    Salmon {
        home_runs_lost: bool,
        away_runs_lost: bool
    },
    PolaritySwitch,
    NightShift {
        batter: bool,
        replacement: Uuid,
        replacement_idx: usize,
        boosts: Vec<f64>
    },
    Fireproof {
        target: Uuid,
    },
    Soundproof {
        resists: Uuid,
        tangled: Uuid,
        decreases: Vec<f64>
    },
    Reverberating {
        batter: Uuid,
    },
    Shelled {
        batter: Uuid
    },
    HitByPitch {
        target: Uuid,
        hbp_type: u8
    },
    PeckedFree {
        player: Uuid
    },
    IffeyJr {
        target: Uuid
    },
    Zap {
        batter: bool
    },
    InstinctWalk {
        third: bool
    },
    BigPeanut {
        target: Uuid
    },
    CharmWalk,
    CharmStrikeout,
    MildPitch,
    MildWalk,
    Repeating {
        batter: Uuid,
    },
    FireEater {
        target: Uuid
    },
    MagmaticHomeRun,
    CrowAmbush,
    TasteTheInfinite {
        target: Uuid
    },
    Inhabiting {
        batter: Uuid,
        inhabit: Uuid,
    },
    BlockedDrain {
        drainer: Uuid,
        target: Uuid,
    },
    Performing {
        overperforming: Vec<Uuid>,
        underperforming: Vec<Uuid>,
    },
    Beaned,
    PouredOver,
    TripleThreat,
    TripleThreatDeactivation {
        home: bool,
        away: bool,
    },
    Swept {
        elsewhere: Vec<Uuid>
    },
    Elsewhere {
        batter: Uuid
    },
    ElsewhereReturn {
        returned: Vec<Uuid>,
        letters: Vec<u8>
    },
    Unscatter {
        unscattered: Vec<Uuid>,
    }
}

impl Event {
    pub fn apply(&self, game: &mut Game, world: &mut World) {
        let repr = self.repr();
        if let Event::BatterUp { .. } = self {
            assert_eq!(repr, String::from("BatterUp"));
        }
        game.events.add(repr.clone());
        match *self {
            Event::BatterUp { batter } => {
                println!("{:?}", world.player(batter).mods);
                let bt = game.scoreboard.batting_team_mut();
                bt.batter = Some(batter);
                if !game.started { game.started = true };
            }
            Event::InningSwitch { inning, top } => {
                if let Weather::Salmon = game.weather {
                    if game.scoreboard.top {
                        let runs_away = game.scoreboard.away_team.score - game.linescore_away[0];
                        game.linescore_away.push(runs_away);
                        game.linescore_away[0] += runs_away;
                    } else {
                        let runs_home = game.scoreboard.home_team.score - game.linescore_home[0];
                        game.linescore_home.push(runs_home);
                        game.linescore_home[0] += runs_home;
                    }
                }
                game.inning = inning;
                game.scoreboard.top = top;
                game.outs = 0;
                game.balls = 0;
                game.strikes = 0;
                game.scoring_plays_inning = 0;
                game.runners = Baserunners::new(game.get_bases(world));
            }
            Event::GameOver => {
                let winning_team = if game.scoreboard.home_team.score > game.scoreboard.away_team.score { game.scoreboard.home_team.id } else { game.scoreboard.away_team.id };
                let losing_team = if game.scoreboard.home_team.score > game.scoreboard.away_team.score { game.scoreboard.away_team.id } else { game.scoreboard.home_team.id };
                if game.day < 99 {
                    world.team_mut(winning_team).wins += 1;
                    world.team_mut(losing_team).losses += 1;
                } else {
                    world.team_mut(winning_team).postseason_wins += 1;
                    world.team_mut(losing_team).postseason_losses += 1;
                }
            }
            Event::Ball => {
                game.balls += 1;
            }
            Event::Strike => {
                game.strikes += 1;
            }
            Event::Foul => {
                game.strikes += 1;
                game.strikes = game.strikes.min(game.get_max_strikes(world) - 1);
            }
            Event::Strikeout | Event::CharmStrikeout => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                let triple_threat_active = world.player(game.pitcher()).mods.has(Mod::TripleThreat)
                    && (game.balls == 3
                        || game.runners.occupied(2)
                        || game.runners.len() == 3);
                if triple_threat_active {
                    game.scoreboard.batting_team_mut().score -= 0.3;
                }
                game.outs += 1;
                game.end_pa();
            }
            Event::Walk | Event::CharmWalk => {
                // maybe we should put batter in the event
                // todo: make a function that returns the current batter
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                game.runners.walk();
                game.runners.add(0, game.batter().unwrap());
                game.score(world);
                game.base_sweep();
                game.end_pa();
            }
            Event::HomeRun => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                upgrade_spicy(game, world);
                let no_runners_on = game.runners.empty();
                game.runners.advance_all(game.get_bases(world));
                game.score(world);
                game.scoreboard.batting_team_mut().score += game.get_run_value();
                game.scoreboard.batting_team_mut().score += world.player(game.batter().unwrap()).get_run_value();
                game.base_sweep();
                if no_runners_on {
                    game.scoring_plays_inning += 1;
                } //this is to make sum sun not break
                game.end_pa();
            }
            Event::BaseHit {
                bases,
                ref runners_after,
            } => {
                let batter = game.batter().unwrap();
                world.player_mut(batter).feed.add(repr.clone());
                upgrade_spicy(game, world);
                game.runners = runners_after.clone();
                game.score(world);
                game.base_sweep();
                game.runners
                    .add(bases - 1, batter);
                game.end_pa();
            }
            Event::GroundOut {
                fielder: _fielder,
                ref runners_after,
            } => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                downgrade_spicy(game, world);
                game.outs += 1;
                game.runners = runners_after.clone();
                game.score(world);
                game.base_sweep();
                game.end_pa();
            }
            Event::Flyout {
                fielder: _fielder,
                ref runners_after,
            } => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                downgrade_spicy(game, world);
                game.outs += 1;
                game.runners = runners_after.clone();
                game.score(world);
                game.base_sweep();
                game.end_pa();
            }
            Event::DoublePlay { ref runners_after } => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                downgrade_spicy(game, world);
                game.outs += 2;
                game.runners = runners_after.clone();
                game.score(world);
                game.base_sweep();
                game.end_pa();
            }
            Event::FieldersChoice { ref runners_after } => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                downgrade_spicy(game, world);
                game.outs += 1;
                game.runners = runners_after.clone();
                game.runners.add(0, game.batter().unwrap());
                game.score(world);
                game.base_sweep();
                game.end_pa();
            }
            Event::BaseSteal {
                runner,
                base_from,
                base_to: _base_to,
            } => {
                if world.player(runner).mods.has(Mod::Blaserunning) {
                    game.scoreboard.batting_team_mut().score += 0.2;
                }
                game.runners.advance(base_from);
                game.score(world);
                game.base_sweep();
            }
            Event::CaughtStealing {
                runner: _runner,
                base_from,
            } => {
                game.runners.remove(base_from);
                game.outs += 1;
            },
            Event::Party {
                target,
                ref boosts
            } => {
                world.player_mut(target).boost(boosts);
            },
            Event::Incineration { target, ref replacement, chain } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Incineration: {}", world.player(target).name);
                println!("Team: {}", world.team(world.player(target).team.unwrap()).name);
                let new_player = replacement.name == "";
                let replacement_id = if new_player {
                    world.add_rolled_player(replacement.clone(), world.player(target).team.unwrap())
                } else {
                    replacement.id
                };
                if let Some(batter) = game.batter() {
                    if batter == target {
                        game.scoreboard.batting_team_mut().batter = Some(replacement_id);
                    }
                } else if target == game.pitcher() {
                    game.scoreboard.pitching_team_mut().pitcher = replacement_id;
                } else if target == game.scoreboard.batting_team().pitcher {
                    game.scoreboard.batting_team_mut().pitcher = replacement_id;
                }
                if new_player {
                    world.replace_player(target, replacement_id);
                } else {
                    world.swap_hall(target, replacement_id);
                }
                if chain.is_some() {
                    world.player_mut(chain.unwrap()).mods.add(Mod::Unstable, ModLifetime::Week);
                }
            },
            Event::Peanut { target, yummy } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Peanut: {}", world.player(target).name);
                println!("Team: {}", world.team(world.player(target).team.unwrap()).name);
                let coeff = if yummy {
                    0.2
                } else {
                    -0.2
                };
                let boosts: Vec<f64> = vec![coeff; 26];
                let player = world.player_mut(target);
                player.boost(&boosts);
            },
            Event::Birds => {},
            Event::Feedback { target1, target2 } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Feedback: {}, {}", world.player(target1).name, world.player(target2).name);
                if let Some(batter) = game.batter() {
                    if batter == target1 {
                        game.assign_batter(target2);
                    } else {
                        game.assign_pitcher(target2);
                    }
                }
                if game.scoreboard.batting_team().pitcher == target2 {
                    game.scoreboard.batting_team_mut().pitcher = target1;
                }
                world.swap(target1, target2);
            },
            Event::Reverb { reverb_type, team, ref changes } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Reverb");
                println!("Team: {}", world.team(team).name);
                world.team_mut(team).apply_reverb_changes(reverb_type, changes);
                if reverb_type != 3 && game.scoreboard.batting_team().id == team {
                    let idx = game.scoreboard.batting_team().batter_index;
                    let world_team = world.team(team);
                    let new_batter = world_team.lineup[idx % world_team.lineup.len()].clone();
                    game.assign_batter(new_batter);
                } else if reverb_type != 2 {
                    if game.scoreboard.pitching_team().id == team {
                        game.assign_pitcher(world.team(team).rotation[game.day % world.team(team).rotation.len()].clone());
                    } else {
                        game.scoreboard.batting_team_mut().pitcher = world.team(team).rotation[game.day % world.team(team).rotation.len()].clone();
                    }
                }
            },
            Event::Blooddrain { drainer, target, stat, siphon: _siphon, siphon_effect } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Blooddrain: {}, {}", world.player(drainer).name, world.player(target).name);
                println!("Drainer team: {}", world.team(world.player(drainer).team.unwrap()).name);
                match siphon_effect {
                    -1 => {
                        let drainer_mut = world.player_mut(drainer);
                        let mut boosts: Vec<f64> = vec![0.0; 26];
                        match stat {
                            0 => {
                                //pitching
                                for i in 8..14 {
                                    boosts[i] = 0.1;
                                }
                            },
                            1 => {
                                //batting
                                for i in 0..8 {
                                    boosts[i] = 0.1;
                                }
                            },
                            2 => {
                                //defense
                                for i in 19..24 {
                                    boosts[i] = 0.1;
                                }
                            },
                            3 => {
                                //baserunning
                                for i in 14..19 {
                                    boosts[i] = 0.1;
                                }
                            },
                            _ => {
                            }
                        }
                        drainer_mut.boost(&boosts);
                    },
                    0 => {
                        game.outs += 1;
                    },
                    1 => {
                        game.outs -= 1;
                    },
                    2 => {
                        game.balls -= 1;
                    },
                    _ => {
                        panic!("wrong siphon effect")
                    }
                }

                let target_mut = world.player_mut(target);
                let mut decreases: Vec<f64> = vec![0.0; 26];
                match stat {
                    0 => {
                        for i in 8..14 {
                            decreases[i] = -0.1;
                        }
                    },
                    1 => {
                        for i in 0..8 {
                            decreases[i] = -0.1;
                        }
                    },
                    2 => {
                        for i in 19..24 {
                            decreases[i] = -0.1;
                        }
                    },
                    3 => {
                        for i in 14..19 {
                            decreases[i] = -0.1;
                        }
                    },
                    _ => {
                    }
                }
                target_mut.boost(&decreases);
            },
            //todo: add win manipulation when we actually have wins
            Event::Sun2 { home_team } => {
                if home_team {
                    game.scoreboard.home_team.score -= 10.0;
                    if game.day > 98 {
                        world.team_mut(game.scoreboard.home_team.id).postseason_wins += 1;
                    } else {
                        world.team_mut(game.scoreboard.home_team.id).wins += 1;
                    }
                } else {
                    game.scoreboard.away_team.score -= 10.0;
                    if game.day > 98 {
                        world.team_mut(game.scoreboard.away_team.id).postseason_wins += 1;
                    } else {
                        world.team_mut(game.scoreboard.away_team.id).wins += 1;
                    }
                }
            }
            Event::BlackHole { home_team } => {
                if home_team {
                    game.scoreboard.home_team.score -= 10.0;
                    if game.day > 98 {
                        world.team_mut(game.scoreboard.away_team.id).postseason_wins -= 1;
                    } else {
                        world.team_mut(game.scoreboard.away_team.id).wins -= 1;
                    }
                } else {
                    game.scoreboard.away_team.score -= 10.0;
                    if game.day > 98 {
                        world.team_mut(game.scoreboard.home_team.id).postseason_wins -= 1;
                    } else {
                        world.team_mut(game.scoreboard.home_team.id).wins -= 1;
                    }
                }
            },
            Event::Salmon { home_runs_lost, away_runs_lost } => {
                if !game.events.has(String::from("Salmon"), if game.scoreboard.top { 3 } else { 2 }) {
                    game.salmon_resets_inning = 0;
                }
                if away_runs_lost {
                    //this whole exercise's goal is
                    //to find the first instance of the inning
                    game.scoreboard.away_team.score -= game.linescore_away[game.linescore_away.len() - 1 - (game.salmon_resets_inning as usize)];
                }
                if home_runs_lost {
                    game.scoreboard.home_team.score -= game.linescore_home[game.linescore_home.len() - 1 - (game.salmon_resets_inning as usize)];
                }
                if !game.scoreboard.top {
                    game.scoreboard.top = true
                } else {
                    game.inning -= 1;
                }
                game.salmon_resets_inning += 1;
            },
            Event::PolaritySwitch => {
                game.polarity = !game.polarity;
            },
            Event::NightShift { batter, replacement, replacement_idx, ref boosts } => {
                if batter {
                    let team = game.scoreboard.batting_team();
                    let active_batter = team.batter.unwrap();
                    let active_batter_order = team.batter_index % world.team(team.id).lineup.len();
                    world.team_mut(team.id).lineup[active_batter_order] = replacement;
                    world.team_mut(team.id).shadows[replacement_idx] = active_batter;
                    world.player_mut(replacement).boost(boosts);
                    let team_mut = game.scoreboard.batting_team_mut();
                    team_mut.batter = Some(replacement);
                } else {
                    let team = game.scoreboard.pitching_team();
                    let active_pitcher = team.pitcher;
                    let active_pitcher_idx = 0; //todo: this only works for one game
                    world.team_mut(team.id).rotation[active_pitcher_idx] = replacement;
                    world.team_mut(team.id).shadows[replacement_idx] = active_pitcher;
                    world.player_mut(replacement).boost(boosts);
                    let team_mut = game.scoreboard.pitching_team_mut();
                    team_mut.pitcher = replacement;
                }
            },
            Event::Fireproof { target: _target } | Event::IffeyJr { target: _target } => {},
            Event::Soundproof { resists: _resists, tangled, ref decreases } => {
                world.player_mut(tangled).boost(decreases);
            },
            Event::Reverberating { batter } => {
                let bt = game.scoreboard.batting_team_mut();
                bt.batter_index -= 1;
                bt.batter = Some(batter);
            }
            Event::Shelled { batter: _batter } | Event::Elsewhere { batter: _batter } => {
                let bt = game.scoreboard.batting_team_mut();
                bt.batter_index += 1;
                if !game.started { game.started = true };
            },
            Event::HitByPitch { target, hbp_type } => {
                let effect = match hbp_type {
                    0 => Some(Mod::Unstable),
                    1 => Some(Mod::Flickering),
                    2 => Some(Mod::Repeating),
                    _ => None
                };
                world.player_mut(target).mods.add(effect.unwrap(), ModLifetime::Week);
                game.runners.walk();
                game.runners.add(0, game.batter().unwrap());
                game.score(world);
                game.base_sweep();
                game.end_pa();
            },
            Event::PeckedFree { player } => {
                world.player_mut(player).mods.remove(Mod::Shelled);
                world.player_mut(player).mods.add(Mod::Superallergic, ModLifetime::Permanent);
            },
            Event::Zap { batter } => {
                if batter {
                    game.strikes -= 1;
                } else {
                    game.balls -= 1;
                }
            },
            Event::InstinctWalk { third } => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                game.runners.walk_instincts(third);
                game.runners.add(if third { 2 } else { 1 }, game.batter().unwrap());
                game.score(world);
                game.base_sweep();
                game.end_pa();
            },
            Event::BigPeanut { target } => {
                println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                println!("Shelled by big peanut: {}", world.player(target).name);
                println!("Team: {}", world.team(world.player(target).team.unwrap()).name);
                world.player_mut(target).mods.add(Mod::Shelled, ModLifetime::Permanent);
            },
            Event::MildPitch => {
                game.balls += 1;
                game.runners.advance_all(1);
                game.score(world);
                game.base_sweep();
            },
            Event::MildWalk => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                game.runners.advance_all(1);
                game.runners.add(0, game.batter().unwrap());
                game.score(world);
                game.base_sweep();
                game.end_pa();
            },
            Event::Repeating { batter } => {
                let bt = game.scoreboard.batting_team_mut();
                bt.batter_index -= 1;
                bt.batter = Some(batter);
            },
            Event::FireEater { target } => {
                world.player_mut(target).mods.add(Mod::Magmatic, ModLifetime::Permanent);
            },
            Event::MagmaticHomeRun => {
                world.player_mut(game.batter().unwrap()).feed.add(repr.clone());
                world.player_mut(game.batter().unwrap()).mods.remove(Mod::Magmatic);
                upgrade_spicy(game, world);
                let no_runners_on = game.runners.empty();
                game.runners.advance_all(game.get_bases(world));
                game.score(world);
                game.scoreboard.batting_team_mut().score += game.get_run_value();
                game.scoreboard.batting_team_mut().score += world.player(game.batter().unwrap()).get_run_value();
                game.base_sweep();
                if no_runners_on {
                    game.scoring_plays_inning += 1;
                } //this is to make sum sun not break
                game.end_pa();
            },
            Event::CrowAmbush => {
                game.outs += 1;
                game.end_pa();
            },
            Event::TasteTheInfinite { target } => {
                world.player_mut(target).mods.add(Mod::Shelled, ModLifetime::Permanent);
            },
            Event::Inhabiting { batter: _batter, inhabit } => {
                let bt = game.scoreboard.batting_team_mut();
                bt.batter = Some(inhabit);
                if !game.started { game.started = true }
            },
            Event::BlockedDrain { drainer: _drainer, target: _target } => {},
            Event::Performing { ref overperforming, ref underperforming } => {
                for &player in overperforming {
                    world.player_mut(player).mods.add(Mod::Overperforming, ModLifetime::Game);
                }
                for &player in underperforming {
                    world.player_mut(player).mods.add(Mod::Underperforming, ModLifetime::Game);
                }
            },
            Event::Beaned => {
                let batter = world.player_mut(game.batter().unwrap());
                if batter.mods.has(Mod::Wired) {
                    batter.mods.remove(Mod::Wired);
                    batter.mods.add(Mod::Tired, ModLifetime::Game);
                } else if batter.mods.has(Mod::Tired) {
                    batter.mods.remove(Mod::Tired);
                } else {
                    batter.mods.add(Mod::Wired, ModLifetime::Game);
                }
            },
            Event::PouredOver => {
                world.player_mut(game.batter().unwrap()).mods.add(Mod::FreeRefill, ModLifetime::Game);
            },
            Event::TripleThreat => {
                world.player_mut(game.scoreboard.home_team.pitcher).mods.add(Mod::TripleThreat, ModLifetime::Permanent);
                world.player_mut(game.scoreboard.away_team.pitcher).mods.add(Mod::TripleThreat, ModLifetime::Permanent);
            },
            Event::TripleThreatDeactivation { home, away } => {
                if home { world.player_mut(game.scoreboard.home_team.pitcher).mods.remove(Mod::TripleThreat); }
                if away { world.player_mut(game.scoreboard.away_team.pitcher).mods.remove(Mod::TripleThreat); }
            },
            Event::Swept { ref elsewhere } => {
                let runners = game.runners.clone();
                for runner in runners.iter() {
                    if world.player(runner.id).mods.has(Mod::Flippers) {
                        game.scoreboard.batting_team_mut().score += world.player(runner.id).get_run_value() + 1.0;
                    }
                }
                game.runners.clear();
                for &runner in elsewhere {
                    println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                    println!("Swept Elsewhere: {}", world.player(runner).name);
                    println!("Team: {}", world.team(world.player(runner).team.unwrap()).name);
                    world.player_mut(runner).mods.add(Mod::Elsewhere, ModLifetime::Permanent);
                    world.player_mut(runner).swept_on = Some(game.day);
                }
            },
            Event::ElsewhereReturn { ref returned, ref letters } => {
                for &player in returned {
                    println!("{} at {}, day {}", world.team(game.scoreboard.away_team.id).name, world.team(game.scoreboard.home_team.id).name, game.day);
                    println!("Returned: {} after {} days", world.player(player).name, game.day - world.player(player).swept_on.unwrap());
                    println!("Team: {}", world.team(world.player(player).team.unwrap()).name);
                    world.player_mut(player).mods.remove(Mod::Elsewhere);
                    world.player_mut(player).swept_on = None;
                }
                for i in 0..letters.len() {
                    let player = returned[i];
                    println!("Scattered: {}, {} letters", world.player(player).name, letters[i]);
                    if letters[i] > 0 {
                        world.player_mut(player).mods.add(Mod::Scattered, ModLifetime::Permanent);
                        world.player_mut(player).scattered_letters = letters[i];
                    }
                }
            }
            Event::Unscatter { ref unscattered } => {
                for &player in unscattered {
                    world.player_mut(player).scattered_letters -= 1;
                    if world.player_mut(player).scattered_letters == 0 {
                        println!("removed scattered from {}", world.player_mut(player).name);
                        world.player_mut(player).mods.remove(Mod::Scattered);
                    }
                }
            }
        }
        game.update_multiplier_data(world);
    }

    //todo: might merge this with a possible future print function
    //btw these don't need to be growable but static lifetimes
    //are annoying
    fn repr(&self) -> String {
        let ev = self.to_string();
        String::from(ev)
    }
}


fn upgrade_spicy(game: &mut Game, world: &mut World) {
    let batter = world.player_mut(game.batter().unwrap());
    if batter.mods.has(Mod::Spicy) && batter.feed.streak_multiple(vec![String::from("BaseHit"), String::from("HomeRun")], -1) == 1 {
        batter.mods.add(Mod::HeatingUp, ModLifetime::Permanent);
    } else if batter.mods.has(Mod::HeatingUp) {
        batter.mods.remove(Mod::HeatingUp);
        batter.mods.add(Mod::RedHot, ModLifetime::Permanent);
    }
}

fn downgrade_spicy(game: &mut Game, world: &mut World) {
     let batter = world.player_mut(game.batter().unwrap());
     if batter.mods.has(Mod::RedHot) {
         batter.mods.remove(Mod::RedHot);
     } else if batter.mods.has(Mod::HeatingUp) {
         batter.mods.remove(Mod::HeatingUp);
     }
}

#[derive(Clone, Debug)]
pub struct Events {
    events: Vec<String>
}

impl Events {
    pub fn new() -> Events {
        Events {
            events: Vec::new()
        }
    }
    pub fn add(&mut self, repr: String) {
        self.events.push(repr);
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn last(&self) -> &String {
        if self.events.len() == 0 {
            panic!("don't call this when the game begins");
        }
        self.events.last().unwrap()
    }
    pub fn has(&self, s: String, limit: i16) -> bool {
        let mut half_innings = 0i16;
        for ev in self.events.iter().rev() {
            if *ev == s {
                return true;
            } else if limit != -1 && *ev == "inningSwitch" {
                if half_innings < limit {
                    half_innings += 1;
                } else {
                    return false;
                }
            }
        }
        false
    }
    pub fn count(&self, s: String, limit: i16) -> u8 {
        let mut half_innings = 0i16;
        let mut counter = 0u8;
        for ev in self.events.iter().rev() {
            if *ev == s {
                counter += 1;
            } else if *ev == "inningSwitch" && limit != -1 {
                if half_innings < limit {
                    half_innings += 1;
                } else {
                    return counter;
                }
            }
        }
        counter
    }
    pub fn streak_multiple(&self, strvec: Vec<String>, limit: i16) -> u8 {
        let mut half_innings = 0i16;
        let mut counter = 0u8;
        for ev in self.events.iter().rev() {
            if *ev == "inningSwitch" && limit != -1 {
                if half_innings < limit {
                    half_innings += 1;
                } else {
                    return counter;
                }
            } else {
		//contains doesn't work
		for s in &strvec {
		    if *ev == *s {
			counter += 1;
		    }
		}
	    }
        }
        counter
    }
}
