mod core;
mod presets;

use std::sync::mpsc::{self, Sender};
use std::thread;

use core::{LifeRule, Engine, EngineCommand, EngineState, Coord};
use eframe::egui;

const CELL_SIZE: f32 = 10.0;
const DEFAULT_ZOOM: f32 = 1.0;

struct CellularAutomataApp {
    simulation: Engine,
    command_tx: Sender<EngineCommand>,
    view_offset: egui::Vec2,
    zoom: f32,
    selected_preset_index: usize,
    presets: Vec<presets::Preset>,
    show_controls: bool,
    placing_mode: bool,
}

impl CellularAutomataApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let simulation = Engine::new();
        let (tx, rx) = mpsc::channel::<EngineCommand>();

        let rule = LifeRule::new();
        let sim_clone = simulation.clone();
        thread::spawn(move || {
            sim_clone.run(std::sync::Arc::new(rule), rx);
        });

        let presets_list = presets::Preset::all();
        let selected_index = presets_list
            .iter()
            .position(|p| p.name == "Глайдер") 
            .unwrap_or(0);

        let initial_preset = presets_list[selected_index].clone();
        simulation.set_world(initial_preset.to_world());

        Self {
            simulation,
            command_tx: tx,
            view_offset: egui::Vec2::ZERO,
            zoom: DEFAULT_ZOOM,
            selected_preset_index: selected_index,
            presets: presets_list,
            show_controls: true,
            placing_mode: false,
        }
    }

    fn send_command(&self, cmd: EngineCommand) {
        let _ = self.command_tx.send(cmd);
    }

    fn screen_to_world(&self, pos: egui::Pos2) -> Coord {
        let x = (pos.x / self.zoom - self.view_offset.x) / CELL_SIZE;
        let y = (pos.y / self.zoom - self.view_offset.y) / CELL_SIZE;
        Coord::new(x as i32, y as i32)
    }

    fn world_to_screen(&self, coord: Coord) -> egui::Pos2 {
        egui::pos2(
            (coord.x as f32 * CELL_SIZE + self.view_offset.x) * self.zoom,
            (coord.y as f32 * CELL_SIZE + self.view_offset.y) * self.zoom,
        )
    }

    fn center_on_world(&mut self) {
        let world = self.simulation.get_world();
        
        if let Some((min, max)) = world.get_bounds() {
            let center_x = (min.x + max.x) as f32 / 2.0;
            let center_y = (min.y + max.y) as f32 / 2.0;
            
            self.view_offset = egui::Vec2::new(
                500.0 - center_x * CELL_SIZE * self.zoom,
                350.0 - center_y * CELL_SIZE * self.zoom,
            );
        } else {
            self.view_offset = egui::Vec2::ZERO;
        }
    }

    fn reset_with_preset(&mut self) {
        if let Some(preset) = self.presets.get(self.selected_preset_index) {
            let _ = self.command_tx.send(EngineCommand::Stop);
            self.simulation.set_world(preset.to_world());
            self.center_on_world();
            let _ = self.command_tx.send(EngineCommand::Pause);
        }
    }

    fn select_preset(&mut self, index: usize) {
        self.selected_preset_index = index;
        
        if self.placing_mode {
            return;
        }

        if let Some(preset) = self.presets.get(index) {
            let _ = self.command_tx.send(EngineCommand::Stop);
            self.simulation.set_world(preset.to_world());
            self.center_on_world();
        }
    }

    fn place_preset_at_cursor(&mut self, cursor_pos: egui::Pos2) {
        if let Some(preset) = self.presets.get(self.selected_preset_index) {
            if preset.cells.is_empty() { return; }
            
            let world_coord = self.screen_to_world(cursor_pos);
            
            let mut min_x = i32::MAX; let mut max_x = i32::MIN;
            let mut min_y = i32::MAX; let mut max_y = i32::MIN;
            for &(x, y) in &preset.cells {
                if x < min_x { min_x = x; } if x > max_x { max_x = x; }
                if y < min_y { min_y = y; } if y > max_y { max_y = y; }
            }
            let center_x = (min_x + max_x) / 2;
            let center_y = (min_y + max_y) / 2;

            let mut world = (*self.simulation.get_world()).clone();
            
            for &(dx, dy) in &preset.cells {
                let coord = Coord::new(world_coord.x + dx - center_x, world_coord.y + dy - center_y);
                world.set_cell(coord, true);
            }
            
            self.simulation.set_world(world);
        }
    }

    fn draw_simulation(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        if response.clicked_by(egui::PointerButton::Primary) && self.placing_mode {
            if let Some(cursor_pos) = response.hover_pos() {
                self.place_preset_at_cursor(cursor_pos);
            }
        }    

        if response.dragged_by(egui::PointerButton::Primary) && !self.placing_mode {
            self.view_offset += response.drag_delta();
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let cursor_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
                let world_pos_before = self.screen_to_world(cursor_pos);
                
                let new_zoom = (self.zoom * (1.0 + scroll * 0.001)).clamp(0.1, 10.0);
                self.zoom = new_zoom;
                
                let world_pos_after = self.screen_to_world(cursor_pos);
                self.view_offset.x += (world_pos_after.x as f32 - world_pos_before.x as f32) * CELL_SIZE * self.zoom;
                self.view_offset.y += (world_pos_after.y as f32 - world_pos_before.y as f32) * CELL_SIZE * self.zoom;
            }
        }

        let painter = ui.painter();
        let world = self.simulation.get_world();
        
        for coord in world.iter_active_cells() {
            let pos = self.world_to_screen(coord);
            let size = CELL_SIZE * self.zoom;
            
            if pos.x < rect.min.x - size || pos.x > rect.max.x + size
                || pos.y < rect.min.y - size || pos.y > rect.max.y + size {
                continue;
            }
            
            let cell_rect = egui::Rect::from_min_size(pos, egui::vec2(size.max(1.0), size.max(1.0)));
            painter.rect_filled(cell_rect, 0.0, egui::Color32::WHITE);
        }

        if self.placing_mode {
            if let Some(cursor_pos) = response.hover_pos() {
                let world_coord = self.screen_to_world(cursor_pos);
                
                if let Some(preset) = self.presets.get(self.selected_preset_index) {
                    if !preset.cells.is_empty() {
                        let mut min_x = i32::MAX; let mut max_x = i32::MIN;
                        let mut min_y = i32::MAX; let mut max_y = i32::MIN;
                        for &(x, y) in &preset.cells {
                            if x < min_x { min_x = x; } if x > max_x { max_x = x; }
                            if y < min_y { min_y = y; } if y > max_y { max_y = y; }
                        }
                        let cx = (min_x + max_x) / 2;
                        let cy = (min_y + max_y) / 2;

                        for &(dx, dy) in &preset.cells {
                            let pos = self.world_to_screen(Coord::new(
                                world_coord.x + dx - cx, 
                                world_coord.y + dy - cy
                            ));
                            let size = CELL_SIZE * self.zoom;
                            let cell_rect = egui::Rect::from_min_size(pos, egui::vec2(size.max(1.0), size.max(1.0)));
                            painter.rect_filled(cell_rect, 0.0, egui::Color32::from_rgba_unmultiplied(100, 255, 100, 100));
                        }
                    }
                }
                
                let cursor_rect = egui::Rect::from_min_size(
                    cursor_pos - egui::vec2(10.0, 10.0),
                    egui::vec2(20.0, 20.0),
                );
                painter.rect_stroke(cursor_rect, 0.0, (2.0, egui::Color32::YELLOW));
            }
        }
    }

    fn draw_control_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("control_panel")
        .default_width(280.0) 
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.heading("Клеточные автоматы");
                    ui.separator();

                ui.group(|ui| {
                    ui.label("Управление");
                    let btn_size = egui::vec2(ui.available_width() * 0.5 - 4.0, 30.0);

                    ui.horizontal(|ui| {
                        if ui.add_sized(btn_size, egui::Button::new("▶️ Run")).clicked() {
                            self.send_command(EngineCommand::Start);
                        }
                        if ui.add_sized(btn_size, egui::Button::new("⏹️ Stop")).clicked() {
                            self.send_command(EngineCommand::Stop);
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.add_sized(btn_size, egui::Button::new("⏭️ Step")).clicked() {
                            self.send_command(EngineCommand::Step);
                        }
                        if ui.add_sized(btn_size, egui::Button::new("🔄 Reset")).clicked() {
                            self.reset_with_preset();
                        }
                    });
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label("Скорость (TPS)");
                    let mut tps = self.simulation.get_tps() as i32;
                    ui.add(egui::Slider::new(&mut tps, 1..=60).text("тиков/сек"));
                    let _ = self.command_tx.send(EngineCommand::SetSpeed(tps as u32));
                });

                ui.separator();

                ui.group(|ui| {
                    let state = self.simulation.get_state();
                    let state_color = match state {
                        EngineState::Running => egui::Color32::GREEN,
                        EngineState::Paused => egui::Color32::YELLOW,
                        EngineState::Stopped => egui::Color32::RED,
                    };
                    ui.colored_label(state_color, format!("Состояние: {:?}", state));
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label("🔍 Навигация");
                    ui.label(format!("Масштаб: {:.1}x", self.zoom));
                    ui.horizontal(|ui| {
                        if ui.button("➖").clicked() { self.zoom = (self.zoom - 0.2).clamp(0.5, 5.0); }
                        if ui.button("➕").clicked() { self.zoom = (self.zoom + 0.2).clamp(0.5, 5.0); }
                        if ui.button("🔄 Сброс").clicked() {
                            self.zoom = DEFAULT_ZOOM;
                            self.view_offset = egui::Vec2::ZERO;
                        }
                    });
                });

                ui.separator();

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("📌 Режим размещения");
                        ui.checkbox(&mut self.placing_mode, "");
                    });
                    
                    if self.placing_mode {
                        ui.colored_label(egui::Color32::GREEN, "АКТИВЕН");
                        ui.small("• Выберите пресет ниже");
                        ui.small("• ЛКМ для размещения");
                        ui.small("• ESC или P для выхода");
                    } else {
                        ui.small("Нажмите P или включите чекбокс");
                    }
                });

                ui.separator();

                ui.group(|ui| {
                    let mode_label = if self.placing_mode { "📍 Пресеты (для размещения)" } else { "📋 Пресеты" };
                    ui.label(mode_label);
                    
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            let mut selected_index = None;
                            for (i, preset) in self.presets.iter().enumerate() {
                                let is_selected = i == self.selected_preset_index;
                                if ui.selectable_label(is_selected, &preset.name).clicked() {
                                    selected_index = Some(i);
                                }
                            }
                            if let Some(index) = selected_index {
                                self.select_preset(index);
                            }
                        });   
                    if let Some(preset) = self.presets.get(self.selected_preset_index) {
                        ui.separator();
                        ui.label(format!("📝 {}", preset.description));
                    }
                });

                ui.separator();

                if ui.button("Центрировать вид").clicked() {
                    self.center_on_world();
                }

                ui.separator();

                if self.show_controls {
                    ui.group(|ui| {
                        ui.label("Горячие клавиши");
                        ui.label("SPACE - Старт/Пауза");
                        ui.label("R - Сброс");
                        ui.label("N - След. шаг");
                        ui.label("H - Скрыть/Показать");
                        ui.label("P - Режим размещения");
                        ui.label("ESC - Выйти из размещения");
                        ui.label("Стрелки - Перемещение");
                        ui.label("+/- - Масштаб");
                        ui.label("Drag - Перемещение");
                        ui.label("Колесо - Зум");
                    });
                }
            });
        });    
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let state = self.simulation.get_state();
            match state {
                EngineState::Running => self.send_command(EngineCommand::Pause),
                _ => self.send_command(EngineCommand::Start),
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) { self.reset_with_preset(); }
        if ctx.input(|i| i.key_pressed(egui::Key::N)) { self.send_command(EngineCommand::Step); }
        if ctx.input(|i| i.key_pressed(egui::Key::H)) { self.show_controls = !self.show_controls; }
        
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.placing_mode = !self.placing_mode;
        }
        if self.placing_mode && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.placing_mode = false;
        }

        let pan_speed = 50.0 / self.zoom;
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) { self.view_offset.y += pan_speed; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) { self.view_offset.y -= pan_speed; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) { self.view_offset.x += pan_speed; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) { self.view_offset.x -= pan_speed; }

        if ctx.input(|i| i.key_pressed(egui::Key::Plus)) { self.zoom = (self.zoom * 1.2).clamp(0.1, 10.0); }
        if ctx.input(|i| i.key_pressed(egui::Key::Minus)) { self.zoom = (self.zoom / 1.2).clamp(0.1, 10.0); }
    }
}

impl eframe::App for CellularAutomataApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Клеточные автоматы | Game of Life");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❓ Помощь").clicked() {
                        self.show_controls = !self.show_controls;
                    }
                });
            });
        });

        self.draw_control_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_simulation(ui);
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Клеточные автоматы",
        options,
        Box::new(|cc| Box::new(CellularAutomataApp::new(cc))),
    )
}