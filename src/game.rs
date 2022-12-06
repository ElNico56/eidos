use std::time::Instant;

use eframe::{
    egui::*,
    epaint::{ahash::HashMap, color::Hsva},
};
use enum_iterator::all;

use crate::{
    field::*,
    function::{Function, FunctionCategory},
    plot::{default_scalar_color, default_vector_color, FieldPlot, MapPlot},
    runtime::Runtime,
    value::Value,
    word::SpellCommand,
    world::World,
};

pub struct Game {
    pub world: World,
    ui_state: UiState,
    spell: SpellState,
    last_time: Instant,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            world: World::default(),
            ui_state: UiState::default(),
            spell: SpellState::default(),
            last_time: Instant::now(),
        }
    }
}

struct UiState {
    fields_visible: HashMap<FieldKind, bool>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            fields_visible: [
                ScalarInputFieldKind::Density.into(),
                VectorOutputFieldKind::Force.into(),
                FieldKind::Uncasted,
            ]
            .map(|kind| (kind, true))
            .into_iter()
            .collect(),
        }
    }
}

#[derive(Clone, Default)]
pub struct SpellState {
    pub spell: Vec<Function>,
    pub staging: Vec<Function>,
}

impl SpellState {
    pub fn command(&mut self, command: SpellCommand) {
        match command {
            SpellCommand::Commit => self.spell.append(&mut self.staging),
            SpellCommand::Disapate => self.staging.clear(),
            SpellCommand::Clear => {
                self.spell.clear();
                self.staging.clear();
            }
        }
    }
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.ui(ui));
        ctx.request_repaint();
    }
}

const BIG_PLOT_SIZE: f32 = 200.0;
const SMALL_PLOT_SIZE: f32 = 100.0;

impl Game {
    fn ui(&mut self, ui: &mut Ui) {
        // Fps
        let now = Instant::now();
        let dt = (now - self.last_time).as_secs_f32();
        self.last_time = now;
        ui.small(format!("{} fps", (1.0 / dt).round()));
        // Calculate fields
        let mut rt = Runtime::default();
        let mut error = None;
        // Calculate spell field
        for function in &self.spell.spell {
            if let Err(e) = rt.call(&mut self.world, *function, true) {
                error = Some(e);
                break;
            }
        }
        self.world.spell_field = rt.top_field().cloned();
        // Execute staging functions
        if error.is_none() {
            for function in &self.spell.staging {
                if let Err(e) = rt.call(&mut self.world, *function, false) {
                    error = Some(e);
                }
            }
        }
        // Draw ui
        Grid::new("fields").show(ui, |ui| {
            // Draw fields
            for field_kind in all::<FieldKind>() {
                ui.toggle_value(
                    self.ui_state
                        .fields_visible
                        .entry(field_kind)
                        .or_insert(false),
                    field_kind.to_string(),
                );
            }
            ui.end_row();
            for field_kind in all::<FieldKind>() {
                if self.ui_state.fields_visible[&field_kind] {
                    self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, field_kind);
                } else {
                    ui.label("");
                }
            }
        });
        // Draw stack
        ui.horizontal_wrapped(|ui| {
            ui.allocate_exact_size(vec2(0.0, SMALL_PLOT_SIZE), Sense::hover());
            for value in &rt.stack {
                match value {
                    Value::Field(field) => self.plot_generic_field(ui, SMALL_PLOT_SIZE, 50, field),
                    Value::Function(_) => {}
                }
            }
        });
        // Draw word buttons
        ui.horizontal_wrapped(|ui| {
            for command in all::<SpellCommand>() {
                if ui.button(command.to_string()).clicked() {
                    self.spell.command(command);
                }
            }
        });
        for category in all::<FunctionCategory>() {
            ui.horizontal_wrapped(|ui| {
                for function in category.functions() {
                    let enabled = error.is_none() && rt.validate_function_use(function).is_ok();
                    if ui
                        .add_enabled(enabled, Button::new(function.to_string()))
                        .clicked()
                    {
                        self.spell.staging.push(function);
                    }
                }
            });
        }
        // Run physics
        self.world.run_physics();
    }
    fn init_plot(&self, size: f32, resolution: usize) -> MapPlot {
        MapPlot::new(&self.world, self.world.player_pos() + Vec2::Y, 5.0)
            .size(size)
            .resolution(resolution)
    }
    pub fn plot_generic_field(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        field: &GenericField,
    ) {
        let plot = self.init_plot(size, resolution);
        match field {
            GenericField::Scalar(ScalarField::Common(CommonField::Uniform(n))) => {
                MapPlot::number_ui(&self.world, ui, size, resolution, *n)
            }
            GenericField::Scalar(field) => plot.ui(ui, field),
            GenericField::Vector(field) => plot.ui(ui, field),
        }
    }
    pub fn plot_field_kind(&self, ui: &mut Ui, size: f32, resolution: usize, kind: FieldKind) {
        let plot = self.init_plot(size, resolution);
        match kind {
            FieldKind::Uncasted => {
                if let Some(field) = &self.world.spell_field {
                    self.plot_generic_field(ui, size, resolution, field)
                } else {
                    ui.allocate_exact_size(vec2(size, size), Sense::hover());
                }
            }
            FieldKind::Typed(GenericFieldKind::Scalar(kind)) => plot.ui(ui, &kind),
            FieldKind::Typed(GenericFieldKind::Vector(kind)) => plot.ui(ui, &kind),
        }
    }
}

impl FieldPlot for ScalarField {
    type Value = f32;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        let h = if t > 0.5 { 0.5 } else { 0.0 };
        let v = 0.7 * (2.0 * t - 1.0).abs() + 0.3;
        let s = (2.0 * t - 1.0).abs();
        Hsva::new(h, s, v, 1.0).into()
    }
}

impl FieldPlot for VectorField {
    type Value = Vec2;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_vector_color(t)
    }
}

impl FieldPlot for GenericScalarFieldKind {
    type Value = f32;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_scalar_field(*self, pos)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_scalar_color(t)
    }
}

impl FieldPlot for GenericVectorFieldKind {
    type Value = Vec2;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_vector_field(*self, pos)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_vector_color(t)
    }
}
