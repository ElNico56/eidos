use std::{collections::HashMap, iter::once};

use eframe::egui::*;
use rapier2d::prelude::*;

use crate::{
    field::*,
    math::rotate,
    person::{NpcId, Person, PersonId},
    physics::PhysicsContext,
    player::Player,
    word::Word,
};

pub struct World {
    pub player: Player,
    pub npcs: HashMap<NpcId, Person>,
    pub objects: HashMap<RigidBodyHandle, Object>,
    pub physics: PhysicsContext,
    pub outputs: OutputFields,
    pub controls: Controls,
}

#[derive(Default)]
pub struct OutputFields {
    pub scalars: HashMap<PersonId, HashMap<ScalarOutputFieldKind, OutputField<ScalarField>>>,
    pub vectors: HashMap<PersonId, HashMap<VectorOutputFieldKind, OutputField<VectorField>>>,
}

impl OutputFields {
    pub fn contains(&self, kind: GenericOutputFieldKind) -> bool {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => self
                .scalars
                .values()
                .any(|fields| fields.contains_key(&kind)),
            GenericOutputFieldKind::Vector(kind) => self
                .vectors
                .values()
                .any(|fields| fields.contains_key(&kind)),
        }
    }
    pub fn remove(&mut self, person_id: PersonId, kind: GenericOutputFieldKind) {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => {
                self.scalars.get_mut(&person_id).unwrap().remove(&kind);
            }
            GenericOutputFieldKind::Vector(kind) => {
                self.vectors.get_mut(&person_id).unwrap().remove(&kind);
            }
        }
    }
    pub fn player_spell(&self, kind: GenericOutputFieldKind) -> Option<&[Word]> {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => self.scalars[&PersonId::Player]
                .get(&kind)
                .map(|output| output.words.as_slice()),
            GenericOutputFieldKind::Vector(kind) => self.vectors[&PersonId::Player]
                .get(&kind)
                .map(|output| output.words.as_slice()),
        }
    }
}

pub struct OutputField<T> {
    pub field: T,
    pub words: Vec<Word>,
}

#[derive(Default)]
pub struct Controls {
    pub x_slider: Option<f32>,
    pub y_slider: Option<f32>,
}

impl Controls {
    pub fn get(&self, kind: ControlKind) -> f32 {
        match kind {
            ControlKind::XSlider => self.x_slider.unwrap_or(0.0),
            ControlKind::YSlider => self.y_slider.unwrap_or(0.0),
        }
    }
}

impl World {
    pub fn new(player: Player) -> Self {
        // Init world
        let mut world = World {
            player,
            npcs: HashMap::new(),
            physics: PhysicsContext::default(),
            objects: HashMap::new(),
            outputs: OutputFields::default(),
            controls: Controls::default(),
        };
        // Add objects
        // Ground
        world.add_object(
            GraphicalShape::HalfSpace(Vec2::Y)
                .offset(Vec2::ZERO)
                .density(3.0),
            RigidBodyBuilder::fixed(),
            |c| c.restitution(0.5),
        );
        // Rock?
        world.add_object(
            GraphicalShape::Circle(1.0).offset(Vec2::ZERO).density(2.0),
            RigidBodyBuilder::dynamic().translation([3.0, 10.0].into()),
            |c| c,
        );
        // Player
        world.player.person.body_handle = world.add_object(
            vec![
                GraphicalShape::Capsule {
                    half_height: 0.25,
                    radius: 0.25,
                }
                .offset(Vec2::ZERO),
                GraphicalShape::Circle(0.3).offset(vec2(0.0, 0.5)),
            ],
            RigidBodyBuilder::dynamic().translation([0.0, 0.5].into()),
            |c| c,
        );
        world
    }
}

pub struct Object {
    pub pos: Pos2,
    pub rot: f32,
    pub shapes: Vec<OffsetShape>,
    pub body_handle: RigidBodyHandle,
}

#[derive(Clone)]
pub struct OffsetShape {
    pub shape: GraphicalShape,
    pub offset: Vec2,
    pub density: f32,
}

impl OffsetShape {
    pub fn contains(&self, pos: Pos2) -> bool {
        self.shape.contains(pos - self.offset)
    }
    pub fn density(self, density: f32) -> Self {
        Self { density, ..self }
    }
}

#[derive(Clone)]
pub enum GraphicalShape {
    Circle(f32),
    #[allow(dead_code)]
    Box(Vec2),
    HalfSpace(Vec2),
    Capsule {
        half_height: f32,
        radius: f32,
    },
}

impl GraphicalShape {
    pub fn offset(self, offset: Vec2) -> OffsetShape {
        OffsetShape {
            shape: self,
            offset,
            density: 1.0,
        }
    }
    pub fn contains(&self, pos: Pos2) -> bool {
        match self {
            GraphicalShape::Circle(radius) => pos.distance(Pos2::ZERO) < *radius,
            GraphicalShape::Box(size) => pos.x.abs() < size.x / 2.0 && pos.y.abs() < size.x / 2.0,
            GraphicalShape::HalfSpace(normal) => pos.y < -normal.x / normal.y * pos.x,
            GraphicalShape::Capsule {
                half_height,
                radius,
            } => {
                pos.x.abs() < *radius && pos.y.abs() < *half_height
                    || pos.distance(pos2(0.0, *half_height)) < *radius
                    || pos.distance(pos2(0.0, -*half_height)) < *radius
            }
        }
    }
}

impl World {
    #[track_caller]
    pub fn person(&self, person_id: PersonId) -> &Person {
        match person_id {
            PersonId::Player => &self.player.person,
            PersonId::Npc(npc_id) => {
                if let Some(person) = self.npcs.get(&npc_id) {
                    person
                } else {
                    panic!("No npc with id {npc_id:?}");
                }
            }
        }
    }
    #[track_caller]
    pub fn person_mut(&mut self, person_id: PersonId) -> &mut Person {
        match person_id {
            PersonId::Player => &mut self.player.person,
            PersonId::Npc(npc_id) => {
                if let Some(person) = self.npcs.get_mut(&npc_id) {
                    person
                } else {
                    panic!("No npc with id {npc_id:?}");
                }
            }
        }
    }
    pub fn find_object_filtered_at(
        &self,
        p: Pos2,
        filter: impl Fn(&Object, &RigidBody) -> bool,
    ) -> Option<(&Object, &OffsetShape)> {
        puffin::profile_function!();
        self.objects.values().find_map(|obj| {
            puffin::profile_function!();
            if !filter(obj, &self.physics.bodies[obj.body_handle]) {
                return None;
            }
            let transformed_point = rotate(p.to_vec2() - obj.pos.to_vec2(), -obj.rot).to_pos2();
            let shape = obj
                .shapes
                .iter()
                .find(|shape| shape.contains(transformed_point))?;
            Some((obj, shape))
        })
    }
    pub fn find_object_at(&self, p: Pos2) -> Option<(&Object, &OffsetShape)> {
        self.find_object_filtered_at(p, |_, _| true)
    }
    pub fn sample_scalar_field(&self, kind: GenericScalarFieldKind, pos: Pos2) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericScalarFieldKind::Input(kind) => self.sample_input_scalar_field(kind, pos),
            GenericScalarFieldKind::Output(kind) => self.sample_output_scalar_field(kind, pos),
        }
    }
    pub fn sample_vector_field(&self, kind: GenericVectorFieldKind, pos: Pos2) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericVectorFieldKind::Input(kind) => self.sample_input_vector_field(kind, pos),
            GenericVectorFieldKind::Output(kind) => self.sample_output_vector_field(kind, pos),
        }
    }
    pub fn sample_input_scalar_field(&self, kind: ScalarInputFieldKind, pos: Pos2) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            ScalarInputFieldKind::Density => self
                .find_object_at(pos)
                .map(|(_, shape)| shape.density)
                .unwrap_or(0.0),
            ScalarInputFieldKind::Elevation => {
                let mut test = pos;
                while test.y > 0.0 {
                    puffin::profile_scope!("elevation test");
                    if self
                        .find_object_filtered_at(test, |_, body| body.body_type().is_fixed())
                        .is_some()
                    {
                        return pos.y - test.y;
                    }
                    test.y -= 0.5;
                }
                pos.y
            }
        }
    }
    pub fn sample_input_vector_field(&self, kind: VectorInputFieldKind, _pos: Pos2) -> Vec2 {
        match kind {}
    }
    pub fn sample_output_scalar_field(&self, kind: ScalarOutputFieldKind, _pos: Pos2) -> f32 {
        match kind {}
    }
    pub fn sample_output_vector_field(&self, kind: VectorOutputFieldKind, pos: Pos2) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        self.outputs
            .vectors
            .iter()
            .fold(Vec2::ZERO, |acc, (person_id, fields)| {
                acc + fields
                    .get(&kind)
                    .map(|output| output.field.sample(self, pos))
                    .unwrap_or_default()
                    * self.person(*person_id).field_scale()
            })
    }
    pub fn people(&self) -> impl Iterator<Item = &Person> {
        self.person_ids_iter().map(|id| self.person(id))
    }
    pub fn person_ids_iter(&self) -> impl Iterator<Item = PersonId> + '_ {
        once(PersonId::Player).chain(self.npcs.keys().copied().map(PersonId::Npc))
    }
    pub fn person_ids(&self) -> Vec<PersonId> {
        self.person_ids_iter().collect()
    }
    pub fn update(&mut self) {
        // Run physics
        let work_done = self.run_physics();
        // Update mana
        for id in self.person_ids() {
            self.person_mut(id).do_work(work_done);
            if !self.person(id).can_cast() {
                self.outputs.scalars.entry(id).or_default().clear();
                self.outputs.vectors.entry(id).or_default().clear();
            }
            if self.outputs.scalars.is_empty() && self.outputs.vectors.is_empty() {
                self.person_mut(id).regen_mana();
            }
        }
    }
}
