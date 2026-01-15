use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::EntityEvent,
        observer::On,
        query::{Changed, Without},
        system::{Commands, Query},
    },
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
    fn on_changed(mut commands: Commands, grid_q: Query<&AttachedCells, Changed<Transform>>) {
        for attached_cells in grid_q {
            for &entity in &attached_cells.0 {
                commands.trigger(UpdateCellPosition { entity });
            }
        }
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
pub struct UpdateCellPosition {
    pub entity: Entity,
}
impl UpdateCellPosition {
    #[allow(clippy::needless_pass_by_value)]
    fn observer(
        event: On<Self>,
        mut grid_cells_q: Query<(&mut Transform, &GridCell, &AttachedToGrid)>,
        grids_q: Query<(&Grid, &Transform), Without<GridCell>>,
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
#[derive(EntityEvent)]
pub struct SnapCellToGrid {
    pub entity: Entity,
}
impl SnapCellToGrid {
    #[allow(clippy::needless_pass_by_value)]
    fn observer(
        event: On<Self>,
        mut commands: Commands,
        mut grid_cells_q: Query<(&mut GridCell, &Transform, &AttachedToGrid)>,
        grids_q: Query<(&Grid, &Transform), Without<GridCell>>,
    ) {
        let Ok((mut cell, cell_transform, grid)) = grid_cells_q.get_mut(event.entity) else {
            return;
        };
        let Ok((grid, grid_transform)) = grids_q.get(grid.0) else {
            return;
        };

        let local_translation =
            (cell_transform.translation - grid_transform.translation).truncate();

        cell.coordinate = (local_translation / (grid.cell_gap + grid.cell_size))
            .round()
            .as_ivec2();

        commands.trigger(UpdateCellPosition {
            entity: event.entity,
        });
    }
}

#[derive(Default)]
pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(UpdateCellPosition::observer)
            .add_observer(SnapCellToGrid::observer);
        app.add_systems(Update, Grid::on_changed);
    }
}
