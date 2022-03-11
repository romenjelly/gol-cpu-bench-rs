use crate::parallelism::{Jobber, Buffer};

pub struct CheckerboardJobber { }

impl<T> Jobber<T, CheckerboardConf<T>> for CheckerboardJobber
    where T: Copy
{
    fn process_job(_buffer: &Buffer<T>, index: usize, conf: &CheckerboardConf<T>) -> T {
        return if (index + (index / conf.width)) % 2 == 0 { conf.color_a } else { conf.color_b };
    }
}

#[derive(Clone, Copy)]
pub struct CheckerboardConf<T>
    where T: Copy + Clone
{
    pub color_a: T,
    pub color_b: T,
    pub width: usize,
}
