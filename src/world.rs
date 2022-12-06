use std::collections::HashMap;

use eframe::egui::*;
use rapier2d::prelude::*;

use crate::{field::*, math::rotate, physics::PhysicsContext};

pub struct World {
    pub player_pos: Pos2,
    pub player: Player,
    pub objects: HashMap<RigidBodyHandle, Object>,
    pub physics: PhysicsContext,
    pub spell_field: Option<GenericField>,
    pub outputs: OutputFields,
}

pub struct Player {
    pub body_handle: RigidBodyHandle,
}

#[derive(Default)]
pub struct OutputFields {
    pub scalars: HashMap<ScalarOutputFieldKind, ScalarField>,
    pub vectors: HashMap<VectorOutputFieldKind, VectorField>,
}

impl Default for World {
    fn default() -> Self {
        // Init world
        let mut world = World {
            player_pos: Pos2::ZERO,
            player: Player {
                body_handle: RigidBodyHandle::default(),
            },
            physics: PhysicsContext::default(),
            objects: HashMap::new(),
            outputs: OutputFields::default(),
            spell_field: None,
        };
        // Add objects
        // Ground
        world.add_object(
            GraphicalShape::HalfSpace(Vec2::Y),
            RigidBodyBuilder::fixed(),
            |c| c.density(3.0),
        );
        // Rock?
        world.add_object(
            GraphicalShape::Circle(1.0),
            RigidBodyBuilder::dynamic().translation([3.0, 10.0].into()),
            |c| c.density(2.0).restitution(1.0),
        );
        // Player
        world.player.body_handle = world.add_object(
            GraphicalShape::Capsule {
                half_height: 0.25,
                radius: 0.25,
            },
            RigidBodyBuilder::dynamic().translation([2.0, 0.5].into()),
            |c| c.density(1.0),
        );
        world
    }
}
pub struct Object {
    pub pos: Pos2,
    pub rot: f32,
    pub shape: GraphicalShape,
    pub density: f32,
    pub shape_offset: Vec2,
    pub body_handle: RigidBodyHandle,
}

#[derive(Clone)]
pub enum GraphicalShape {
    Circle(f32),
    Box(Vec2),
    HalfSpace(Vec2),
    Capsule { half_height: f32, radius: f32 },
}

impl GraphicalShape {
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
    pub fn find_object_at(&self, p: Pos2) -> Option<&Object> {
        self.objects.values().find(|obj| {
            let transformed_point =
                rotate(p.to_vec2() - obj.pos.to_vec2() - obj.shape_offset, -obj.rot).to_pos2();
            obj.shape.contains(transformed_point)
        })
    }
    pub fn sample_scalar_field(&self, kind: GenericScalarFieldKind, pos: Pos2) -> f32 {
        match kind {
            GenericScalarFieldKind::Input(kind) => self.sample_input_scalar_field(kind, pos),
            GenericScalarFieldKind::Output(kind) => self.sample_output_scalar_field(kind, pos),
        }
    }
    pub fn sample_vector_field(&self, kind: GenericVectorFieldKind, pos: Pos2) -> Vec2 {
        match kind {
            GenericVectorFieldKind::Input(kind) => self.sample_input_vector_field(kind, pos),
            GenericVectorFieldKind::Output(kind) => self.sample_output_vector_field(kind, pos),
        }
    }
    pub fn sample_input_scalar_field(&self, kind: ScalarInputFieldKind, pos: Pos2) -> f32 {
        match kind {
            ScalarInputFieldKind::Density => self
                .find_object_at(pos)
                .map(|obj| obj.density)
                .unwrap_or(0.0),
        }
    }
    pub fn sample_input_vector_field(&self, kind: VectorInputFieldKind, _pos: Pos2) -> Vec2 {
        match kind {}
    }
    pub fn sample_output_scalar_field(&self, kind: ScalarOutputFieldKind, _pos: Pos2) -> f32 {
        match kind {}
    }
    pub fn sample_output_vector_field(&self, kind: VectorOutputFieldKind, pos: Pos2) -> Vec2 {
        self.outputs
            .vectors
            .get(&kind)
            .map(|field| field.sample(self, pos))
            .unwrap_or_default()
    }
}
