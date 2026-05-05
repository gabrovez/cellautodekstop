use super::life_rule::LifeRule;
use super::world::World;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineCommand {
    Start,
    Stop,
    Pause,
    Step,
    SetSpeed(u32),
}

#[derive(Clone)]
pub struct Engine {
    world: Arc<RwLock<Arc<World>>>,
    next_world: Arc<Mutex<World>>,
    state: Arc<Mutex<EngineState>>,
    tick_count: Arc<Mutex<u64>>,
    tps: Arc<Mutex<u32>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            world: Arc::new(RwLock::new(Arc::new(World::new()))),
            next_world: Arc::new(Mutex::new(World::new())),
            state: Arc::new(Mutex::new(EngineState::Stopped)),
            tick_count: Arc::new(Mutex::new(0)),
            tps: Arc::new(Mutex::new(10)),
        }
    }

    pub fn get_world(&self) -> Arc<World> {
        self.world.read().unwrap().clone()
    }

    pub fn set_world(&self, world: World) {
        *self.world.write().unwrap() = Arc::new(world);
    }

    pub fn get_state(&self) -> EngineState {
        *self.state.lock().unwrap()
    }

    pub fn get_tps(&self) -> u32 {
        *self.tps.lock().unwrap()
    }

    pub fn set_tps(&self, new_tps: u32) {
        let tps = new_tps.clamp(1, 1000);
        let mut current_tps = self.tps.lock().unwrap();
        *current_tps = tps;
    }

    pub fn step(&self, rule: &LifeRule) {
        let current_arc = {
            let lock = self.world.read().unwrap();
            lock.clone()
        };

        let mut next = self.next_world.lock().unwrap();
        rule.apply_into(&*current_arc, &mut *next);

        {
            let mut lock = self.world.write().unwrap();
            *lock = Arc::new(std::mem::replace(&mut *next, World::new()));
        }

        let mut tick = self.tick_count.lock().unwrap();
        *tick += 1;
    }

    pub fn reset_tick_count(&self) {
        let mut tick = self.tick_count.lock().unwrap();
        *tick = 0;
    }

    pub fn run(
        &self,
        rule: Arc<LifeRule>,
        command_rx: std::sync::mpsc::Receiver<EngineCommand>,
    ) -> thread::JoinHandle<()> {
        let sim = self.clone();

        thread::spawn(move || {
            loop {
                while let Ok(cmd) = command_rx.try_recv() {
                    match cmd {
                        EngineCommand::Start => {
                            let mut state = sim.state.lock().unwrap();
                            *state = EngineState::Running;
                        }
                        EngineCommand::Stop => {
                            let mut state = sim.state.lock().unwrap();
                            *state = EngineState::Stopped;
                            sim.reset_tick_count();
                        }
                        EngineCommand::Pause => {
                            let mut state = sim.state.lock().unwrap();
                            *state = EngineState::Paused;
                        }
                        EngineCommand::Step => {
                            sim.step(rule.as_ref());
                        }
                        EngineCommand::SetSpeed(tps) => {
                            sim.set_tps(tps);
                        }
                    }
                }

                let state = sim.get_state();
                
                if state == EngineState::Running {
                    let start = Instant::now();
                    sim.step(rule.as_ref());
                    
                    let tps = sim.get_tps();
                    let tick_duration = Duration::from_millis(1000 / tps as u64);
                    let elapsed = start.elapsed();

                    if elapsed < tick_duration {
                        thread::sleep(tick_duration - elapsed);
                    }
                } else {
                    thread::sleep(Duration::from_millis(1)); 
                }
            }
        })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}