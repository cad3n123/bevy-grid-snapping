use bevy::{
    app::{App, Plugin},
    ecs::{component::Component, entity::Entity, event::EntityEvent, observer::On, system::Query},
    math::{IVec2, Vec2, Vec3},
    transform::components::Transform,
};

// Components
#[derive(Component)]
#[require(AttachedCells, Transform)]
pub struct Grid {
    pub cell_size: Vec2,
    pub cell_gap: Vec2,
}
impl Grid {
    fn get_cell_position(&self, cell: &GridCell) -> Vec3 {
        (cell.coordinate.as_vec2() * (self.cell_size + self.cell_gap)).extend(0.)
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct GridCell {
    pub coordinate: IVec2,
}

// Relationships
#[derive(Component, Default)]
#[relationship_target(relationship = AttachedToGrid)]
pub struct AttachedCells(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = AttachedCells)]
pub struct AttachedToGrid(pub Entity);

// Events
#[derive(EntityEvent)]
pub struct CellToSnap {
    pub entity: Entity,
}
impl CellToSnap {
    #[allow(clippy::needless_pass_by_value)]
    fn observer(
        event: On<Self>,
        mut grid_cells_q: Query<(&mut Transform, &GridCell, &AttachedToGrid)>,
        grids_q: Query<(&Grid, &Transform)>,
    ) {
        let Ok((mut cell_transform, cell, grid)) = grid_cells_q.get_mut(event.entity) else {
            return;
        };
        let Ok((grid, grid_transform)) = grids_q.get(grid.0) else {
            return;
        };
        cell_transform.translation = grid_transform.translation + grid.get_cell_position(cell);
    }
}

#[derive(Default)]
pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(CellToSnap::observer);
    }
}
