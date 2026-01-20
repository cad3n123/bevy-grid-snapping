#[cfg(feature = "debug")]
use bevy::{
    app::Last,
    ecs::{
        hierarchy::Children,
        query::{Added, Has, Or, With},
    },
};
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
    math::{UVec2, Vec2, Vec3},
    transform::components::Transform,
};

#[derive(Default)]
pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(UpdateCellPosition::observer)
            .add_observer(SnapCellToGrid::observer)
            .add_observer(TrySnapCellToGrid::observer);
        app.add_systems(Update, Grid::on_changed);

        #[cfg(feature = "debug")]
        app.add_systems(Last, Grid::debug_on_changed);
    }
}

// Components
#[derive(Component)]
#[require(AttachedCells, Transform)]
pub struct Grid {
    pub cell_size: Vec2,
    pub cell_gap: Vec2,
    pub offset: Vec2,
    pub dimensions: (Option<u32>, Option<u32>),
}

impl Grid {
    fn get_cell_position(&self, cell: &GridCell) -> Vec3 {
        (cell.coordinate.as_vec2() * (self.cell_size + self.cell_gap) + self.offset).extend(0.)
    }
    fn get_cell_coordinate(
        &self,
        grid_transform: &Transform,
        cell_transform: &Transform,
        round_to_nearest: bool,
    ) -> Option<UVec2> {
        let local_translation =
            (cell_transform.translation - grid_transform.translation).truncate() - self.offset;

        let int_result = (local_translation / (self.cell_gap + self.cell_size))
            .round()
            .as_ivec2();

        if round_to_nearest {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            return Some(UVec2::new(
                if let Some(dim_x) = self.dimensions.0 {
                    int_result.x.clamp(0, dim_x as i32)
                } else {
                    int_result.x.max(0)
                } as u32,
                if let Some(dim_y) = self.dimensions.1 {
                    int_result.y.clamp(0, dim_y as i32)
                } else {
                    int_result.y.max(0)
                } as u32,
            ));
        }

        if !int_result.x.is_negative()
            && !int_result.y.is_negative()
            && self.is_coordinate_valid(int_result.as_uvec2())
        {
            Some(int_result.as_uvec2())
        } else {
            None
        }
    }
    fn is_coordinate_valid(&self, coordinate: UVec2) -> bool {
        self.dimensions.0.is_none_or(|width| coordinate.x < width)
            && self.dimensions.1.is_none_or(|height| coordinate.y < height)
    }
    fn on_changed(mut commands: Commands, grid_q: Query<&AttachedCells, Changed<Transform>>) {
        for attached_cells in grid_q {
            for &entity in &attached_cells.0 {
                commands.trigger(UpdateCellPosition { entity });
            }
        }
    }

    #[cfg(feature = "debug")]
    fn debug_on_changed(
        mut commands: Commands,
        grid_q: Query<(Entity, &Self, Option<&Children>), Or<(Added<Self>, Changed<Self>)>>,
        cell_outlines_q: Query<Entity, With<DebugCellOutline>>,
    ) {
        use bevy::{
            color::{Alpha, Color, palettes::tailwind::GREEN_400},
            sprite::Sprite,
            utils::default,
        };
        for (grid_e, grid, grid_children) in grid_q {
            // Despawn any cell outlines
            if let Some(grid_children) = grid_children {
                for &child in grid_children {
                    if cell_outlines_q.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
            }
            // Spawn new cell outlines
            let dimensions = (
                grid.dimensions.0.unwrap_or(100),
                grid.dimensions.1.unwrap_or(100),
            );

            commands.entity(grid_e).with_children(|parent| {
                for x in 0..dimensions.0 {
                    for y in 0..dimensions.1 {
                        #[allow(clippy::cast_precision_loss)]
                        let transform = Transform::from_xyz(
                            x as f32 * (grid.cell_size.x + grid.cell_gap.x) + grid.offset.x,
                            y as f32 * (grid.cell_size.y + grid.cell_gap.y) + grid.offset.y,
                            0.0,
                        );

                        parent.spawn((
                            DebugCellOutline,
                            Sprite {
                                color: Color::from(GREEN_400.with_alpha(0.2)),
                                custom_size: Some(grid.cell_size - 0.2),
                                ..default()
                            },
                            transform,
                        ));
                    }
                }
            });
        }
    }
}
#[cfg(feature = "debug")]
#[derive(Component)]
struct DebugCellOutline;

#[derive(Component, Default)]
#[require(Transform)]
pub struct GridCell {
    pub coordinate: UVec2,
}

// Relationships
#[derive(Component, Default)]
#[relationship_target(relationship = AttachedToGrid)]
pub struct AttachedCells(Vec<Entity>);

#[derive(Component)]
#[require(GridCell)]
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
        let cell_position = grid.get_cell_position(cell);
        cell_transform.translation = grid_transform
            .translation
            .with_z(cell_transform.translation.z)
            + cell_position;
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

        let Some(coordinate) = grid.get_cell_coordinate(grid_transform, cell_transform, true)
        else {
            return;
        };

        cell.coordinate = coordinate;

        commands.trigger(UpdateCellPosition {
            entity: event.entity,
        });
    }
}
#[derive(EntityEvent)]
pub struct TrySnapCellToGrid {
    pub entity: Entity,
}
impl TrySnapCellToGrid {
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

        let Some(coordinate) = grid.get_cell_coordinate(grid_transform, cell_transform, false)
        else {
            return;
        };

        cell.coordinate = coordinate;

        commands.trigger(UpdateCellPosition {
            entity: event.entity,
        });
    }
}
