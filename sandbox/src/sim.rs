use uuid::Uuid;

use crate::{entities::{World, Player}, events::Event, formulas, mods::{Mod, Mods}, rng::Rng, Game, Weather};

pub trait Plugin {
    fn tick(&self, _game: &Game, _world: &World, _rng: &mut Rng) -> Option<Event> {
        None
    }
}

pub struct Sim<'a> {
    plugins: Vec<Box<dyn Plugin>>,
    pub world: &'a mut World,
    pub rng: &'a mut Rng,
}

impl<'a> Sim<'a> {
    pub fn new(world: &'a mut World, rng: &'a mut Rng) -> Sim<'a> {
        Sim {
            world,
            rng,
            plugins: vec![
                Box::new(PregamePlugin),
                Box::new(InningStatePlugin),
                Box::new(InningEventPlugin),
                Box::new(BatterStatePlugin),
                Box::new(WeatherPlugin),
                Box::new(ElsewherePlugin),
                Box::new(PartyPlugin),
                Box::new(FloodingPlugin),
                Box::new(ModPlugin),
                Box::new(StealingPlugin),
                Box::new(BasePlugin),
            ],
        }
    }
    pub fn next(&mut self, game: &Game) -> Event {
        for plugin in self.plugins.iter() {
            if let Some(event) = plugin.tick(game, &self.world, &mut self.rng) {
                return event;
            }
        }

        panic!("uhhh")
    }
}

enum PitchOutcome {
    Ball,
    StrikeSwinging,
    StrikeLooking,
    Foul,
    GroundOut {
        fielder: Uuid,
        advancing_runners: Vec<Uuid>
    },
    Flyout { 
        fielder: Uuid,
        advancing_runners: Vec<Uuid>
    },
    DoublePlay { runner_out: u8 },
    FieldersChoice { runner_out: u8 },
    HomeRun,
    Triple { advancing_runners: Vec<Uuid> },
    Double { advancing_runners: Vec<Uuid> },
    Single { advancing_runners: Vec<Uuid> },
    Quadruple { advancing_runners: Vec<Uuid> }
}

struct BasePlugin;
impl Plugin for BasePlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let max_balls = game.get_max_balls(world);
        let max_strikes = game.get_max_strikes(world);
        // let max_outs = 3;

        let last_strike = (game.strikes + 1) >= max_strikes;

        Some(match do_pitch(world, game, rng) {
            PitchOutcome::Ball => {
                if (game.balls + 1) < max_balls {
                    Event::Ball
                } else {
                    if world.player(game.batter().unwrap()).mods.has(Mod::BaseInstincts) && rng.next() < 0.2 {
                        Event::InstinctWalk { third: rng.next() * rng.next() < 0.5 }
                    } else {
                        Event::Walk
                    }
                }
            }
            PitchOutcome::StrikeSwinging => {
                if last_strike {
                    Event::Strikeout
                } else {
                    Event::Strike
                }
            }
            PitchOutcome::StrikeLooking => {
                if last_strike {
                    if world.team(game.scoreboard.batting_team().id).mods.has(Mod::ONo) && game.balls == 0 {
                        Event::Foul
                    } else {
                        Event::Strikeout
                    }
                } else {
                    Event::Strike
                }
            }
            PitchOutcome::Foul => Event::Foul,
            PitchOutcome::GroundOut { fielder, advancing_runners } => {
                let mut new_runners = game.runners.clone();
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::GroundOut {
                    fielder,
                    runners_after: new_runners,
                }
            },
            PitchOutcome::Flyout { fielder, advancing_runners } => {
                let mut new_runners = game.runners.clone();
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::Flyout {
                    fielder,
                    runners_after: new_runners,
                }
            },
            PitchOutcome::DoublePlay { runner_out } => {
                let mut new_runners = game.runners.clone();
                new_runners.remove(runner_out);
                new_runners.advance_all(1);
                Event::DoublePlay {
                    runners_after: new_runners
                }
            },
            PitchOutcome::FieldersChoice { runner_out } => {
                let mut new_runners = game.runners.clone();
                new_runners.remove(runner_out);
                new_runners.advance_all(1);
                Event::FieldersChoice {
                    runners_after: new_runners
                }
            },

            PitchOutcome::HomeRun => Event::HomeRun,

            // todo: there may be a subtle bug here since we don't sweep the runners after the forced advance
            // runner [1, 0], double, then we're at [3, 2], 3 *should* get swept and *then* 2 should get to advance to 3...
            PitchOutcome::Triple { advancing_runners } => {
                let mut new_runners = game.runners.clone();
                new_runners.advance_all(3);
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::BaseHit {
                    bases: 3,
                    runners_after: new_runners,
                }
            },

            PitchOutcome::Double { advancing_runners } => {
                let mut new_runners = game.runners.clone();
                new_runners.advance_all(2);
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::BaseHit {
                    bases: 2,
                    runners_after: new_runners,
                }
            },

            PitchOutcome::Single { advancing_runners } => {
                let mut new_runners = game.runners.clone();
                new_runners.advance_all(1);
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::BaseHit {
                    bases: 1,
                    runners_after: new_runners,
                }
            },

            PitchOutcome::Quadruple { advancing_runners }=> {
                let mut new_runners = game.runners.clone();
                new_runners.advance_all(4);
                new_runners.advance_if(|runner| advancing_runners.contains(&runner.id));
                Event::BaseHit {
                    bases: 4,
                    runners_after: new_runners,
                }
            },
        })
    }
}

fn do_pitch(world: &World, game: &Game, rng: &mut Rng) -> PitchOutcome {
    let pitcher = world.player(game.pitcher());
    let batter = world.player(game.batter().unwrap());
    let ruleset = world.season_ruleset; //todo: can we fold this into multiplier_data?

    let is_flinching = game.strikes == 0 && batter.mods.has(Mod::Flinch);

    let multiplier_data = &game.compute_multiplier_data(world);

    let is_strike = rng.next() < formulas::strike_threshold(pitcher, batter, is_flinching, ruleset, multiplier_data);
    let does_swing = if !is_flinching {
        rng.next() < formulas::swing_threshold(pitcher, batter, is_strike, ruleset, multiplier_data)
    } else {
        false
    };

    if !does_swing {
        if is_strike {
            return PitchOutcome::StrikeLooking;
        } else {
            return PitchOutcome::Ball;
        }
    }

    let does_contact = rng.next() < formulas::contact_threshold(pitcher, batter, is_strike, ruleset, multiplier_data);
    if !does_contact {
        return PitchOutcome::StrikeSwinging;
    }

    let is_foul = rng.next() < formulas::foul_threshold(pitcher, batter, ruleset, multiplier_data);
    if is_foul {
        return PitchOutcome::Foul;
    }

    let out_defender_id = game.pick_fielder(world, rng.next());
    let out_defender = world.player(out_defender_id);

    let is_out = rng.next() > formulas::out_threshold(pitcher, batter, out_defender, ruleset, multiplier_data);
    if is_out {
        let fly_defender_id = game.pick_fielder(world, rng.next());
        let fly_defender = world.player(fly_defender_id);

        let is_fly = rng.next() < formulas::fly_threshold(batter, pitcher, ruleset, multiplier_data);
        if is_fly {
            let mut advancing_runners = Vec::new();
            if game.outs == 2 {
                return PitchOutcome::Flyout {
                    fielder: fly_defender_id,
                    advancing_runners
                };
            }
            for baserunner in game.runners.iter() {
                let base_from = baserunner.base;
                let runner_id = baserunner.id.clone();
                let runner = world.player(runner_id);

                if rng.next() < formulas::flyout_advancement_threshold(runner, base_from, ruleset, multiplier_data) {
                    advancing_runners.push(runner_id);
                }
            }
            return PitchOutcome::Flyout {
                fielder: fly_defender_id,
                advancing_runners
            };
        }

        let ground_defender_id = game.pick_fielder(world, rng.next());
        let mut advancing_runners = Vec::new();
        if game.outs == 2 {
            return PitchOutcome::GroundOut {
                fielder: ground_defender_id,
                advancing_runners
            };
        }

        if !game.runners.empty() {
            let dp_roll = rng.next();
            if game.runners.occupied(0) {
                if game.outs < 2 && dp_roll < formulas::double_play_threshold(batter, pitcher, out_defender, ruleset, multiplier_data) {
                    return PitchOutcome::DoublePlay {
                        runner_out: game.runners.pick_runner(rng.next())
                    };
                } else {
                    let sac_roll = rng.next();
                    if sac_roll < formulas::groundout_sacrifice_threshold(batter, ruleset, multiplier_data) {
                        for baserunner in game.runners.iter() {
                            let runner_id = baserunner.id.clone();
                            let runner = world.player(runner_id);
                            if rng.next() < formulas::groundout_advancement_threshold(runner, out_defender, ruleset, multiplier_data) {
                                advancing_runners.push(runner_id);
                            }
                        }
                        return PitchOutcome::GroundOut {
                            fielder: ground_defender_id,
                            advancing_runners
                        };
                    } else {
                        return PitchOutcome::FieldersChoice {
                            runner_out: game.runners.pick_runner_fc()
                        }
                    }
                }
            }
            for baserunner in game.runners.iter() {
                let runner_id = baserunner.id.clone();
                let runner = world.player(runner_id);
                if rng.next() < formulas::groundout_advancement_threshold(runner, out_defender, ruleset, multiplier_data) {
                    advancing_runners.push(runner_id);
                }
            }
        }
        return PitchOutcome::GroundOut {
            fielder: ground_defender_id,
            advancing_runners
        };
    }

    let is_hr = rng.next() < formulas::hr_threshold(pitcher, batter, ruleset, multiplier_data);
    if is_hr {
        return PitchOutcome::HomeRun;
    }

    let hit_defender_id = game.pick_fielder(world, rng.next());
    let hit_defender = world.player(hit_defender_id);
    let double_roll = rng.next();
    let triple_roll = rng.next();
    let mut quadruple_roll = 1.0;
    if game.get_bases(world) == 5 {
        quadruple_roll = rng.next();
    }

    let mut advancing_runners = Vec::new();
    for baserunner in game.runners.iter() {
        let runner_id = baserunner.id.clone();
        let runner = world.player(runner_id);

        if rng.next() < formulas::hit_advancement_threshold(runner, hit_defender, ruleset, multiplier_data) {
            advancing_runners.push(runner_id);
        }
    }

    if quadruple_roll < formulas::quadruple_threshold(pitcher, batter, hit_defender, ruleset, multiplier_data) {
        return PitchOutcome::Quadruple {
            advancing_runners
        };
    }

    if triple_roll < formulas::triple_threshold(pitcher, batter, hit_defender, ruleset, multiplier_data) {
        return PitchOutcome::Triple {
            advancing_runners
        };
    }
    if double_roll < formulas::double_threshold(pitcher, batter, hit_defender, ruleset, multiplier_data) {
        return PitchOutcome::Double {
            advancing_runners
        };
    }

    PitchOutcome::Single {
        advancing_runners
    }
}

struct BatterStatePlugin;
impl Plugin for BatterStatePlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let batting_team = game.scoreboard.batting_team();
        if game.batter().is_none() {
            let idx = batting_team.batter_index;
            let team = world.team(batting_team.id);
            let first_batter = if !game.started {
                true
            } else if idx == 0 && game.inning == 1 && game.events.last() == "InningSwitch" {
                true
            } else {
                false
            };
            let inning_begin = !first_batter && game.events.last() == "InningSwitch";
            let prev = if first_batter { team.lineup[0].clone() } else { team.lineup[(idx - 1) % team.lineup.len()].clone() };
            //todo: improve this
            if !first_batter && !inning_begin && world.player(prev).mods.has(Mod::Reverberating) && rng.next() < 0.2 { //rough estimate
                return Some(Event::Reverberating { batter: prev });
            } else if !first_batter && !inning_begin && world.player(prev).mods.has(Mod::Repeating) && (game.events.last() == "BaseHit" || game.events.last() == "HomeRun") {
                if let Weather::Reverb = game.weather {
                    return Some(Event::Repeating { batter: prev });
                }
            }
            let batter = team.lineup[idx % team.lineup.len()].clone();
            if world.player(batter).mods.has(Mod::Shelled) {
                return Some(Event::Shelled { batter });
            } else if world.player(batter).mods.has(Mod::Elsewhere) {
                return Some(Event::Elsewhere { batter });
            } else if world.player(batter).mods.has(Mod::Haunted) && rng.next() < 0.2 {
                let inhabit = world.random_hall_player(rng);
                return Some(Event::Inhabiting { batter, inhabit });
            }
            Some(Event::BatterUp { batter })
        } else {
            None
        }
    }
}

struct InningStatePlugin;
impl Plugin for InningStatePlugin {
    fn tick(&self, game: &Game, _world: &World, _rng: &mut Rng) -> Option<Event> {
        if game.outs < 3 {
            return None;
        }

        let lead = if (game.scoreboard.away_team.score - game.scoreboard.home_team.score).abs() < 0.01 {
            0
        } else if game.scoreboard.away_team.score > game.scoreboard.home_team.score {
            1
        } else {
            -1
        }; // lol floats
        if game.inning >= 9 && (lead == -1 || !game.scoreboard.top && lead == 1) {
            return Some(Event::GameOver);
        }

        if game.scoreboard.top {
            Some(Event::InningSwitch {
                inning: game.inning,
                top: false,
            })
        } else {
            Some(Event::InningSwitch {
                inning: game.inning + 1,
                top: true,
            })
        }
    }
}

struct StealingPlugin;
impl Plugin for StealingPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let steal_defender_id = game.pick_fielder(world, rng.next());
        let steal_defender = world.player(steal_defender_id);

        // todo: can we refactor `Baserunners` in a way where this sort of iteration is more natural
        for base in (0..game.get_bases(world)).rev() {
            if let Some(runner_id) = game.runners.at(base) {
                if game.runners.can_advance(base) {
                    let runner = world.player(runner_id);
                    let should_attempt =
                        rng.next() < formulas::steal_attempt_threshold(runner, steal_defender);
                    if should_attempt {
                        let success =
                            rng.next() < formulas::steal_success_threshold(runner, steal_defender);

                        if success {
                            return Some(Event::BaseSteal {
                                runner: runner_id,
                                base_from: base,
                                base_to: base + 1,
                            });
                        } else {
                            return Some(Event::CaughtStealing {
                                runner: runner_id,
                                base_from: base,
                            });
                        }
                    }
                }
            }
        }

        None
    }
}

//exclusion: "all", "current", "playing"
fn poll_for_mod(game: &Game, world: &World, a_mod: Mod, exclusion: &str) -> Vec<Uuid> {
    let home_team = &game.scoreboard.home_team;
    let away_team = &game.scoreboard.away_team;

    //not good that runners.runners is accessed directly
    let home_lineup = if !game.scoreboard.top && exclusion == "playing" { [vec![game.batter().unwrap()], game.runners.iter().map(|r| r.id).collect()].concat() } else { world.team(home_team.id).lineup.clone() };
    let home_pitcher = if exclusion != "all" { 
        if !game.scoreboard.top && exclusion == "playing" {
            Vec::new()
        } else {
            vec![home_team.pitcher.clone()] 
        }
    } else { 
        world.team(home_team.id).rotation.clone() 
    };
    let away_lineup = if game.scoreboard.top && exclusion == "playing" { [vec![game.batter().unwrap()], game.runners.iter().map(|r| r.id).collect()].concat() } else { world.team(away_team.id).lineup.clone() };
    let away_pitcher = if exclusion != "all" { 
        if game.scoreboard.top && exclusion == "playing" {
            Vec::new()
        } else {
            vec![away_team.pitcher.clone()] 
        }
    } else { 
        world.team(away_team.id).rotation.clone() 
    };

    let mut players = vec![home_lineup, home_pitcher, away_lineup, away_pitcher].concat();

    players.retain(|player| world.player(*player).mods.has(a_mod));

    players
}

struct WeatherPlugin;
impl Plugin for WeatherPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let fort = 0.0;
        let ruleset = world.season_ruleset;
        match game.weather {
            Weather::Sun => None,
            Weather::Eclipse => {
                //todo: add fortification
                let fire_eaters = poll_for_mod(game, world, Mod::FireEater, "playing");
                let incin_roll = rng.next();
                //todo: the Fire Eater picker prioritizes unstable players
                if fire_eaters.len() > 0 {
                    for fe in fire_eaters {
                        if rng.next() < 0.002 { //estimate
                            return Some(Event::FireEater { target: fe });
                        }
                    }
                }
                let target = game.pick_player_weighted(world, rng.next(), |&uuid| !game.runners.contains(uuid), true);
                let unstable_check = world.player(target).mods.has(Mod::Unstable) && incin_roll < 0.002; //estimate
                let regular_check = incin_roll < 0.00045 - 0.0004 * fort;
                if unstable_check || regular_check { //estimate
                    if world.player(target).mods.has(Mod::Fireproof) || world.team(world.player(target).team.unwrap()).mods.has(Mod::Fireproof) {
                        return Some(Event::Fireproof { target });
                    }
                    let minimized = poll_for_mod(game, world, Mod::Minimized, "all");
                    if minimized.len() > 0 {
                        if minimized.len() > 1 { 
                            //assuming that there's
                            //no more than one legendary item of each kind
                            //at any point in the sim
                            todo!()
                        } else {
                            if world.player(target).team.unwrap() == world.player(minimized[0]).team.unwrap() && world.player(minimized[0]).mods.has(Mod::Minimized) {
                                return Some(Event::IffeyJr { target });
                            }
                        }
                    }
                    let chain: Option<Uuid> = None;
                    if unstable_check {
                        let chain_target = game.pick_player_weighted(world, rng.next(), |&uuid| world.player(uuid).team.unwrap() != world.player(target).team.unwrap(), false);
                        let chain = if world.player(chain_target).mods.has(Mod::Stable) { None } else { Some(chain_target) };//assumption
                    }
                    let replacement = if world.player(target).mods.has(Mod::Squiddish) {
                        world.player(world.random_hall_player(rng)).clone()
                    } else {
                        Player::new(rng)
                    };
                    Some(Event::Incineration { 
                        target,
                        replacement,
                        chain
                    })
                } else {
                    None
                }
            },
            Weather::Peanuts => {
                if rng.next() < 0.000002 { //estimate
                    //this is maybe not rng compliant
                    let target = game.pick_player_weighted(world, rng.next(), |&_uuid| true, true); //theory
                    Some(Event::BigPeanut {
                        target
                    })
                } else if rng.next() < 0.0006 - 0.00055 * fort {
                    //idk if runners can have a reaction
                    //but this is assuming it's the same as incins
                    let target = game.pick_player_weighted(world, rng.next(), |&uuid| !game.runners.contains(uuid), true);
                    Some(Event::Peanut {
                        target,
                        yummy: false
                    })
                } else if world.player(game.batter().unwrap()).mods.has(Mod::HoneyRoasted) && rng.next() < 0.0076 {
                    //todo: we don't know
                    rng.next();
                    Some(Event::TasteTheInfinite { target: game.pick_fielder(world, rng.next()) })
                } else if world.player(game.pitcher()).mods.has(Mod::HoneyRoasted) && rng.next() < 0.0061 {
                    Some(Event::TasteTheInfinite { target: game.batter().unwrap() })
                } else {
                    None
                }
            },
            Weather::Birds => {
                //rough estimate
                if rng.next() < 0.03 {
                    return Some(Event::Birds);
                } //todo: this is definitely not rng accurate
                
                let shelled_players = poll_for_mod(game, world, Mod::Shelled, "all");
                for player in shelled_players {
                    //estimate, not sure how accurate this is
                    let shelled_roll = rng.next();
                    if world.team(world.player(player).team.unwrap()).mods.has(Mod::BirdSeed) && shelled_roll < 0.001 || shelled_roll < 0.00015 { //estimate. lmao at bird seed
                        return Some(Event::PeckedFree { player });
                    }
                }
                None
            },
            Weather::Feedback => {
                let is_batter = rng.next() < (9.0 / 14.0);
                let feedback_roll = rng.next();
                let batter = game.batter().unwrap();
                let pitcher = game.pitcher();

                let mut target1_opt: Option<Uuid> = None;
                let mut target2_opt: Option<Uuid> = None;

                //the old implementation checked super flickering players first, then flickering, then regular. 
                //the new one just checks the batter first.
                //This might or might not be wrong
                if is_batter {
                    let feedback_check = world.player(batter).mods.has(Mod::SuperFlickering) && feedback_roll < 0.055
                        || world.player(batter).mods.has(Mod::Flickering) && feedback_roll < 0.02
                        || feedback_roll < 0.0001 - 0.0001 * fort;

                    if feedback_check {
                        let target2_raw = game.pick_fielder(world, rng.next());
                    
                        target1_opt = Some(batter);
                        target2_opt = Some(target2_raw);
                    }
                } else {
                    let feedback_check = world.player(pitcher).mods.has(Mod::SuperFlickering) && feedback_roll < 0.055
                        || world.player(pitcher).mods.has(Mod::Flickering) && feedback_roll < 0.02
                        || feedback_roll < 0.0001 - 0.0001 * fort;

                    if feedback_check {   
                        let batting_team = world.team(game.scoreboard.batting_team().id);
                        let idx = (rng.next() * (batting_team.rotation.len() as f64)).floor() as usize;
                        let target2_raw = batting_team.rotation[idx];
                        target1_opt = Some(pitcher);
                        target2_opt = Some(target2_raw);
                    }
                }
                if target1_opt.is_some() {
                    let target1 = target1_opt.unwrap();
                    let target2 = target2_opt.unwrap();
                    if world.player(target1).mods.has(Mod::Soundproof) {
                        let decreases = roll_random_boosts(rng, 0.0, -0.05, true);
                        Some(Event::Soundproof {
                            resists: target1,
                            tangled: target2,
                            decreases
                        })
                    } else if world.player(target2).mods.has(Mod::Soundproof) {
                        let decreases = roll_random_boosts(rng, 0.0, -0.05, true);
                        Some(Event::Soundproof {
                            resists: target2,
                            tangled: target1,
                            decreases
                        })
                    } else {
                        Some(Event::Feedback {
                            target1,
                            target2
                        })
                    }
                } else {
                    None
                }
            },
            Weather::Reverb => {
                //estimate
                if rng.next() < 0.00003 {
                    let reverb_type_roll = rng.next();
                    let reverb_type = if reverb_type_roll < 0.09 {
                        0u8
                    } else if reverb_type_roll < 0.55 {
                        1u8
                    } else if reverb_type_roll < 0.95 {
                        2u8
                    } else {
                        3u8
                    };
                    let team_id = if rng.next() < 0.5 {
                        game.scoreboard.home_team.id
                    } else {
                        game.scoreboard.away_team.id
                    };

                    let mut gravity_players: Vec<usize> = vec![];

                    let team = world.team(team_id.clone());

                    for i in 0..team.lineup.len() {
                        if world.player(team.lineup[i]).mods.has(Mod::Gravity) {
                            gravity_players.push(i);
                        }
                    }
                    for i in 0..team.rotation.len() {
                        if world.player(team.rotation[i]).mods.has(Mod::Gravity) {
                            gravity_players.push(i + team.lineup.len());
                        }
                    } //todo: make this prettier

                    let changes = team.roll_reverb_changes(rng, reverb_type, &gravity_players);
                    
                    Some(Event::Reverb {
                        reverb_type,
                        team: team_id,
                        changes
                    })
                } else {
                    None
                }
            },
            Weather::Blooddrain => {
                let drain_threshold = if ruleset < 16 { 
                    0.00065 - 0.001 * fort 
                } else {
                    0.00125 - 0.00125 * fort
                };
                let siphon_threshold = 0.0025;
                let siphons = poll_for_mod(game, world, Mod::Siphon, "playing");
                let drain_roll = rng.next();
                if drain_roll < drain_threshold || siphons.len() > 0 && drain_roll < siphon_threshold { //rulesets
                    let mut drainer: Uuid;
                    let mut target: Uuid;
                    let siphon = drain_roll > drain_threshold;
                    //siphon code
                    if siphon {
                        let siphon_player = siphons[rng.index(siphons.len())];
                        let active_target = rng.next() < 0.5;
                        if active_target {
                            target = if siphon_player == game.batter().unwrap() { game.pitcher() } else { game.batter().unwrap() };
                        } else {
                            let target_roll = rng.next();
                            if world.player(siphon_player).team.unwrap() == game.scoreboard.batting_team().id {
                                target = game.pick_fielder(world, target_roll);
                            } else {
                                let hitter = if game.runners.empty() {
                                    game.batter().unwrap()
                                } else {
                                    game.pick_player_weighted(world, rng.next(), |&uuid| uuid == game.batter().unwrap() || game.runners.contains(uuid), true)
                                };
                                target = hitter
                            }
                        }
                        drainer = siphon_player;
                    } else {
                        let fielding_team_drains = rng.next() < 0.5;
                        let is_atbat = rng.next() < 0.5;
                        if is_atbat {
                            drainer = if fielding_team_drains { game.pitcher() } else { game.batter().unwrap() };
                            target = if fielding_team_drains { game.batter().unwrap() } else { game.pitcher() };
                        } else {
                            let fielder_roll = rng.next();
                            let fielder = game.pick_fielder(world, fielder_roll);
                            let hitter = if game.runners.empty() {
                                game.batter().unwrap()
                            } else {
                                game.pick_player_weighted(world, rng.next(), |&uuid| uuid == game.batter().unwrap() || game.runners.contains(uuid), true)
                            };
                            drainer = if fielding_team_drains { fielder } else { hitter };
                            target = if fielding_team_drains { hitter } else { fielder };
                        }
                    }
                    if world.team(world.player(target).team.unwrap()).mods.has(Mod::Sealant) {
                        Some(Event::BlockedDrain { drainer, target })
                    } else {
                        let siphon_effect_roll = if siphon { rng.next() } else { 0.0 };
                        let siphon_effect = if siphon_effect_roll < 0.35 {
                            -1
                        } else {
                            if world.player(drainer).team.unwrap() == game.scoreboard.batting_team().id {
                                if game.outs > 0 && siphon_effect_roll < 0.5 {//wild guesstimates
                                    1
                                } else {
                                    -1
                                }
                            } else {
                                if game.balls > 0 && siphon_effect_roll < 0.8 {
                                    2
                                } else {
                                    0
                                }
                            }
                        };
                        Some(Event::Blooddrain {
                            drainer,
                            target,
                            stat: (rng.next() * 4.0).floor() as u8,
                            siphon,
                            siphon_effect
                        })
                    }
                } else {
                    None
                }
            },
            Weather::Sun2 => {
                if game.scoreboard.home_team.score > 9.99 { //ugh
                    Some(Event::Sun2 { home_team: true })
                } else if game.scoreboard.away_team.score > 9.99 {
                    Some(Event::Sun2 { home_team: false })
                } else {
                    None
                }
            },
            Weather::BlackHole => {
                if game.scoreboard.home_team.score > 9.99 {
                    Some(Event::BlackHole { home_team: true })
                } else if game.scoreboard.away_team.score > 9.99 {
                    Some(Event::BlackHole { home_team: false })
                } else {
                    None
                }
            },
            Weather::Coffee => {
                if rng.next() < 0.02 - 0.012 * fort {
                    Some(Event::Beaned)
                } else {
                    None
                }
            },
            Weather::Coffee2 => {
                if rng.next() < 0.01875 - 0.0075 * fort && !world.player(game.batter().unwrap()).mods.has(Mod::FreeRefill) {
                    Some(Event::PouredOver)
                } else {
                    None
                }
            },
            Weather::Coffee3 => None,
            Weather::Flooding => None,
            Weather::Salmon => None,
            Weather::PolarityPlus | Weather::PolarityMinus => {
                if rng.next() < 0.035 - 0.025 * fort {
                    Some(Event::PolaritySwitch)
                } else {
                    None
                }
            },
            Weather::SunPointOne | Weather::SumSun => None,
            Weather::Night => {
                if rng.next() < 0.01 { //estimate
                    let batter = rng.next() < 0.5;
                    let shadows = if batter { &world.team(game.scoreboard.batting_team().id).shadows } else { &world.team(game.scoreboard.pitching_team().id).shadows };
                    let replacement_idx = (rng.next() * shadows.len() as f64).floor() as usize;
                    let replacement = shadows[replacement_idx as usize];
                    let boosts = roll_random_boosts(rng, 0.0, 0.2, false);
                    Some(Event::NightShift { batter, replacement, replacement_idx, boosts })
                } else {
                    None
                }
            }
        }
    }
}

fn roll_random_boosts(rng: &mut Rng, base: f64, threshold: f64, exclude_press: bool) -> Vec<f64> {
    let mut boosts: Vec<f64> = Vec::new();
    //does Tangled decrease press or cinn???
    let stat_number = if exclude_press { 25 } else { 26 };
    for _ in 0..stat_number {
        boosts.push(base + rng.next() * threshold);
    }
    boosts
}

struct InningEventPlugin;
impl Plugin for InningEventPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let activated = |event: &str| game.events.has(String::from(event), 1);
        //note: inning events happen after the inning switch
        //they also happen after batter up apparently (?)
        if !activated("TripleThreatDeactivation") && game.inning == 4 && game.scoreboard.top {
            let home_pitcher_deactivated = world.player(game.scoreboard.home_team.pitcher).mods.has(Mod::TripleThreat) && rng.next() < 0.333;
            let away_pitcher_deactivated = world.player(game.scoreboard.away_team.pitcher).mods.has(Mod::TripleThreat) && rng.next() < 0.333;
            if home_pitcher_deactivated || away_pitcher_deactivated {
                return Some(Event::TripleThreatDeactivation { home: home_pitcher_deactivated, away: away_pitcher_deactivated });
            }
        }
        if let Weather::Salmon = game.weather {
            let away_team_scored = game.linescore_away.last().unwrap().abs() > 0.01;
            let home_team_scored = if !game.scoreboard.top { false } else { game.linescore_home.last().unwrap().abs() > 0.01 };
            if game.events.len() > 0 && game.events.last() == "InningSwitch" && (away_team_scored || home_team_scored) {
                let salmon_activated = rng.next() < 0.1375;
                if salmon_activated {
                    let runs_lost = rng.next() < 0.675; //rough estimate
                    if runs_lost {
                        if away_team_scored && home_team_scored {
                            let double_runs_lost = rng.next() < 0.2; //VERY rough estimate
                            if double_runs_lost {
                                return Some(Event::Salmon { away_runs_lost: true, home_runs_lost: true });
                            }
                            let home_runs_lost = rng.next() < 0.5;
                            return Some(Event::Salmon { away_runs_lost: !home_runs_lost, home_runs_lost });
                        }
                        if away_team_scored {
                            return Some(Event::Salmon { away_runs_lost: true, home_runs_lost: false });
                        }
                        return Some(Event::Salmon { away_runs_lost: false, home_runs_lost: true });
                    }
                    return Some(Event::Salmon { away_runs_lost: false, home_runs_lost: false });
                }
            }
            return None;
        }
        None
    }
}

struct ModPlugin;
impl Plugin for ModPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        //this whole function? rulesets
        let batter = game.batter().unwrap();
        let batter_mods = &world.player(batter).mods;
        let batter_team_mods = &world.team(game.scoreboard.batting_team().id).mods;
        let pitcher = game.pitcher();
        let pitcher_mods = &world.player(pitcher).mods;
        let pitcher_team_mods = &world.team(game.scoreboard.pitching_team().id).mods;
        if batter_team_mods.has(Mod::Electric) && game.strikes > 0 && rng.next() < 0.2 {
            return Some(Event::Zap { batter: true });
        } else if pitcher_team_mods.has(Mod::Electric) && game.balls > 0 && rng.next() < 0.2 {
            return Some(Event::Zap { batter: false });
        } else if pitcher_mods.has(Mod::DebtU) && !batter_mods.has(Mod::Unstable) && rng.next() < 0.02 { //estimate
            return Some(Event::HitByPitch { target: batter, hbp_type: 0 });
        } else if pitcher_mods.has(Mod::RefinancedDebt) && !batter_mods.has(Mod::Flickering) && rng.next() < 0.02 { //estimate
            return Some(Event::HitByPitch { target: batter, hbp_type: 1 });
        } else if pitcher_mods.has(Mod::ConsolidatedDebt) && !batter_mods.has(Mod::Repeating) && rng.next() < 0.02 { //estimate
            return Some(Event::HitByPitch { target: batter, hbp_type: 2 });
        } else if pitcher_mods.has(Mod::FriendOfCrows) {
            if let Weather::Birds = game.weather {
                if rng.next() < 0.0255 {
                    return Some(Event::CrowAmbush);
                }
            }
        }
        if rng.next() < 0.005 && pitcher_mods.has(Mod::Mild) {
            if game.balls == 3 {
                return Some(Event::MildWalk);
            } else {
                return Some(Event::MildPitch);
            }
        } else if game.balls == 0 && game.strikes == 0 {
            let myst = 0.0;
        let charm_threshold = if world.season_ruleset == 18 {
                0.014 + 0.006 * myst
            } else {
                0.015 + 0.02 * myst
            };
            if batter_mods.has(Mod::Charm) && rng.next() < charm_threshold {
                return Some(Event::CharmWalk);
            } else if pitcher_mods.has(Mod::Charm) && rng.next() < charm_threshold {
                return Some(Event::CharmStrikeout);
            } else if batter_mods.has(Mod::Magmatic) {
                //this makes it so magmatic cannot activate on non 0-0 counts
                //edge cases are, well, not impossible
                rng.next();
                return Some(Event::MagmaticHomeRun);
            }
        }
        None
    }
}

struct PregamePlugin;
impl Plugin for PregamePlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        if !game.started {
            let activated = |event: &str| game.events.has(String::from(event), -1);
            if let Weather::Coffee3 = game.weather {
                if !activated("TripleThreat") {
                    return Some(Event::TripleThreat);
                }
            }
            let mut overperforming = vec![];
            let mut underperforming = vec![];
            //todo: make this a separate event
            let superyummy = poll_for_mod(game, world, Mod::Superyummy, "current");
            if superyummy.len() > 0 {
                if let Weather::Peanuts = game.weather {
                    overperforming = [overperforming, superyummy].concat();
                } else {
                    underperforming = [underperforming, superyummy].concat();
                }
            }
            
            let perk = poll_for_mod(game, world, Mod::Perk, "current");
            if perk.len() > 0 {
                if let Weather::Coffee = game.weather {
                    overperforming = [overperforming, perk].concat();
                } else if let Weather::Coffee2 = game.weather {
                    overperforming = [overperforming, perk].concat();
                } else if let Weather::Coffee3 = game.weather {
                    overperforming = [overperforming, perk].concat();
                }
            }

            //other performing code here
            if !activated("Performing") && (overperforming.len() > 0 || underperforming.len() > 0) {
                Some(Event::Performing { overperforming, underperforming })
            } else {
                None
            }
        } else {
            None
        }
    }
}

struct PartyPlugin;
impl Plugin for PartyPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let party_roll = rng.next();
        let party_threshold = if world.season_ruleset < 20 { 0.0055 } else { 0.00525 };
        if party_roll < party_threshold {
            let party_team = if rng.next() < 0.5 { world.team(game.scoreboard.home_team.id) } else { world.team(game.scoreboard.away_team.id) };
            if party_team.partying {
                let lineup_length = party_team.lineup.len();
                let rotation_length = party_team.rotation.len();
                let index = rng.index(lineup_length + rotation_length);
                let target = if index < lineup_length { //guessing
                    party_team.lineup[index]
                } else {
                    party_team.rotation[index - lineup_length]
                };
                let party_number = if world.player(target).mods.has(Mod::LifeOfTheParty) { 0.048 } else { 0.04 };
                let boosts = roll_random_boosts(rng, party_number, party_number, true);
                Some(Event::Party { target, boosts })
            } else {
                None
            }
        } else {
            None
        }
    }
}

struct FloodingPlugin;
impl Plugin for FloodingPlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        if let Weather::Flooding = game.weather {
            let fort = 0.0;
            let flooding_threshold = match world.season_ruleset {
                11..14 => 0.019 - 0.02 * fort,
                14..17 => 0.013 - 0.012 * fort,
                17 => 0.015 - 0.012 * fort,
                18..24 => 0.016 - 0.012 * fort,
                _ => 0.0,
            };
            if rng.next() < flooding_threshold {
                let mut elsewhere: Vec<Uuid> = Vec::new();
                for runner in game.runners.iter() {
                    //todo: flooding threshold depends on myst and fort
                    if rng.next() < 0.1 {
                        elsewhere.push(runner.id);
                    }
                }
                Some(Event::Swept { elsewhere })
            } else {
                None
            }
        } else {
            None
        }
    }
}

struct ElsewherePlugin;
impl Plugin for ElsewherePlugin {
    fn tick(&self, game: &Game, world: &World, rng: &mut Rng) -> Option<Event> {
        let elsewhere_return_threshold = match world.season_ruleset {
            11 => 0.001,
            12 => 0.000575,
            13..18 => 0.0004,
            18..24 => 0.00035,
            _ => 0.0
        };
        let lineup = &world.team(game.scoreboard.batting_team().id).lineup;
        let rotation = &world.team(game.scoreboard.batting_team().id).rotation;
        let mut returned = Vec::new(); //ugh
        let mut letters = Vec::new();
        for &player in lineup {
            if world.player(player).mods.has(Mod::Elsewhere) && rng.next() < elsewhere_return_threshold {
                returned.push(player);
                let scattered = world.player(player);
                let time_elsewhere = game.day - scattered.swept_on.unwrap();
                let mut player_letters = 0u8;
                if time_elsewhere > 18 {
                    for i in 0..(scattered.name.len() - 1) {
                        //todo: we don't know how it works, do we?
                        rng.next();
                        //theory
                        if rng.next() < (time_elsewhere as f64) / 100.0 {
                            player_letters += 1;
                        }
                    }
                }
                letters.push(player_letters);
            }
        }
        for &player in rotation {
            if world.player(player).mods.has(Mod::Elsewhere) && rng.next() < elsewhere_return_threshold {
                returned.push(player);
                let scattered = world.player(player);
                let time_elsewhere = game.day - scattered.swept_on.unwrap();
                let mut player_letters = 0u8;
                if time_elsewhere > 18 {
                    for i in 0..(scattered.name.len() - 1) {
                        //todo: we don't know how it works, do we?
                        rng.next();
                        //theory
                        if rng.next() < (time_elsewhere as f64) / 100.0 {
                            player_letters += 1;
                        }
                    }
                }
                letters.push(player_letters);
            }
        }
        //that last part is to stop it from rolling twice
        if returned.len() > 0 && game.events.last() != "ElsewhereReturn" {
            Some(Event::ElsewhereReturn { returned, letters })
        } else {
            let unscatter_threshold = match world.season_ruleset {
                11 | 12 => 0.00061,
                13 => 0.0005,
                14..17 => 0.0004,
                17..20 => 0.00042,
                20 | 21 => 0.000485,
                22 | 23 => 0.000495,
                _ => 0.0
            };
            let mut unscattered = Vec::new();
            for &player in lineup {
                if world.player(player).mods.has(Mod::Scattered) && rng.next() < unscatter_threshold {
                    unscattered.push(player);
                }
            }
            for &player in rotation {
                if world.player(player).mods.has(Mod::Scattered) && rng.next() < unscatter_threshold {
                    unscattered.push(player);
                }
            }
            if unscattered.len() > 0 && game.events.last() != "Unscattered" {
                Some(Event::Unscatter { unscattered })
            } else {
                None
            }
        }
    }
}
