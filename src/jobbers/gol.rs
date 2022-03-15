use std::fmt::Debug;
use crate::parallelism::{Jobber, Buffer, Executor};
use std::io::{Write, stdout};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GolCell {
    Alive,
    Dead,
}

impl GolCell {
    pub fn is_alive(&self) -> bool {
        return self == &GolCell::Alive;
    }
}

impl From<bool> for GolCell {
    fn from(cell: bool) -> Self {
        match cell {
            true => GolCell::Alive,
            false => GolCell::Dead,
        }
    }
}

impl Debug for GolCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alive => write!(f, "X"),
            Self::Dead => write!(f, "."),
        }
    }
}

impl Into<char> for GolCell {
    fn into(self) -> char {
        return match self {
            GolCell::Alive => '▓',
            GolCell::Dead => '░',
        };
    }
}

pub struct GameOfLifeJobber { }

const NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (-1,  1),
    ( 0,  1),
    ( 1,  1),
    ( 1,  0),
    ( 1, -1),
    ( 0, -1),
    (-1, -1),
    (-1,  0),
];

impl GameOfLifeJobber {
    fn get_neighbor_count(pos: (usize, usize), buffer: &Buffer<GolCell>) -> usize {
        return NEIGHBOR_OFFSETS
            .iter()
            .filter_map(|(offset_x, offset_y)| buffer.at_2d_i32((
                pos.0 as i32 + offset_x,
                pos.1 as i32 + offset_y,
            )))
            .filter(|neighbor_cell| neighbor_cell.is_alive())
            .count();

        /*
        // Alternative "traditional" impl
        let mut neighbor_count = 0_usize;
        for (offset_x, offset_y) in NEIGHBOR_OFFSETS {
            if let Some(cell) = buffer.at_2d_i32((
                pos.0 as i32 + offset_x,
                pos.1 as i32 + offset_y,
            )) {
                if cell.is_alive() {
                    neighbor_count += 1;
                }
            }
        }
        return neighbor_count;
        */
    }
}

impl Jobber<GolCell, ()> for GameOfLifeJobber {
    fn process_job(buffer: &Buffer<GolCell>, index: usize, _conf: &()) -> GolCell {
        let cell_pos = buffer.index_to_pos_2d(index);
        let cell = buffer.data[index];
        let neighbor_count = GameOfLifeJobber::get_neighbor_count(cell_pos, buffer);
        
        /*
        // More verbose/explicit, but slightly slower for some reason
        return match cell {
            GolCell::Alive => neighbor_count == 2 || neighbor_count == 3,
            GolCell::Dead => neighbor_count == 3,
        }.into();
        */
        return ((neighbor_count == 3) || (neighbor_count == 2 && cell.is_alive())).into();
    }
}

pub const GOL_VIS_CHAR_WIDTH: usize = 2_usize;

pub struct ExecutorGolVis {
    frame_interval: f32,
}

impl ExecutorGolVis {
    pub fn new(target_framerate: usize) -> Self {
        Self {
            frame_interval: 1_f32 / target_framerate as f32,
        }
    }
}

impl Executor<GolCell, ()> for ExecutorGolVis
{
    fn compute(&self, in_buffer: Buffer<GolCell>, out_buffer: &mut [GolCell], conf: ()) -> Buffer<GolCell> {
        // Very bad way to do this since it adds the processing and printing overhead to the loop
        // but it avoids interior mutability of the Executor; but maybe it should be &mut anyway
        std::thread::sleep(std::time::Duration::from_secs_f32(self.frame_interval));

        for index in 0..(in_buffer.len()) {
            out_buffer[index] = GameOfLifeJobber::process_job(&in_buffer, index, &conf);
        }

        let mut stdout = stdout();
        let output: String = out_buffer
            .chunks_exact(in_buffer.dims.0)
            .map(|cells| cells
                .iter()
                .map(|cell| std::iter::repeat(Into::<char>::into(*cell)).take(GOL_VIS_CHAR_WIDTH))
                .flatten()
                .chain(['\n'])
            )
            .flatten()
            .collect()
            ;
        stdout.write_all(output.trim().as_bytes()).expect("Error writing to console");
        return in_buffer;
    }
}
