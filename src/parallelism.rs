use std::cell::RefCell;
use std::cmp::Ordering;
use std::time::Instant;
use std::marker::PhantomData;
use std::{sync::Arc, thread::JoinHandle};
use std::thread;
use crossbeam_queue::SegQueue;

#[derive(Clone, Debug)]
pub struct Buffer<T> {
    pub data: Box<[T]>,
    pub dims: (usize, usize, usize),
}

impl<T> Buffer<T>
    where T: Copy
{
    pub fn from_value(length: usize, value: T) -> Self {
        Self {
            data: vec![value; length].into_boxed_slice(),
            dims: (length, 1, 1),
        }
    }

    pub fn from_value_2d(dimensions: (usize, usize), value: T) -> Self {
        Self {
            data: vec![value; dimensions.0 * dimensions.1].into_boxed_slice(),
            dims: (dimensions.0, dimensions.1, 1),
        }
    }

    pub fn from_value_3d(dimensions: (usize, usize, usize), value: T) -> Self {
        Self {
            data: vec![value; dimensions.0 * dimensions.1 * dimensions.2].into_boxed_slice(),
            dims: dimensions,
        }
    }

    pub fn from_vec(vec: Vec<T>) -> Self {
        let len = vec.len();
        Self {
            data: vec.into_boxed_slice(),
            dims: (len, 1, 1),
        }
    }

    pub fn len(&self) -> usize {
        // self.dims.0 * self.dims.1 * self.dims.2
        self.data.len()
    }

    pub fn dims_1d(&self) -> usize {
        self.dims.0
    }

    pub fn dims_2d(&self) -> (usize, usize) {
        (self.dims.0, self.dims.1)
    }

    pub fn dims_3d(&self) -> (usize, usize, usize) {
        (self.dims.0, self.dims.1, self.dims.2)
    }

    pub fn index_to_pos_2d(&self, index: usize) -> (usize, usize) {
        (index % self.dims.0, index / self.dims.0)
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        return self.data.get(index);
    }

    pub fn at_unchecked(&self, index: usize) -> &T {
        return &self.data[index];
    }

    pub fn at_2d(&self, pos: (usize, usize)) -> Option<&T> {
        let index = pos.0 + pos.1 * self.dims.0;
        return self.data.get(index);
    }

    pub fn at_2d_i32(&self, pos: (i32, i32)) -> Option<&T> {
        if pos.0 < 0 || pos.0 >= self.dims.0 as i32 { return None }
        if pos.1 < 0 || pos.1 >= self.dims.1 as i32 { return None }
        let index = pos.0 as usize + pos.1 as usize * self.dims.0;
        return self.data.get(index);
    }

    pub fn at_2d_unchecked(&self, pos: (usize, usize)) -> &T {
        let index = pos.0 + pos.1 * self.dims.0;
        return &self.data[index];
    }

    /*
    pub fn at_3d(pos: (usize, usize, usize), dimensions: (usize, usize, usize)) -> &T {
       todo!()
    }
    */
}

unsafe impl<T> Send for Buffer<T> {}
unsafe impl<T> Sync for Buffer<T> {}


// TODO: Add "Sleep" command that makes the jobber use thread::sleep() instead of thread::yield_now() until job is received
// Will be useful to not fry the CPU whilst between jobs
pub enum JobSignal<T, TConf> {
    Work(JobDescriptor<T, TConf>),
    Death,
}

pub struct JobDescriptor<T, TConf> {
    buffer: Arc<Buffer<T>>,
    conf: Arc<TConf>,

    offset: usize,
    count: usize,
    out_buffer: Vec<T>,
}

pub struct JobResult<T> {
    buffer: Vec<T>,
    count: usize,

    offset: usize,
}

impl<T, TConf> From<JobDescriptor<T, TConf>> for JobResult<T> {
    fn from(descriptor: JobDescriptor<T, TConf>) -> Self {
        Self {
            buffer: descriptor.out_buffer,
            count: descriptor.count,
            offset: descriptor.offset,
        }
    }
}

impl<T> Ord for JobResult<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset)
    }
}

impl<T> PartialOrd for JobResult<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for JobResult<T> {
    fn eq(&self, other: &Self) -> bool {
        self.offset.eq(&other.offset)
    }
}

impl<T> Eq for JobResult<T> { }

pub trait Jobber<T, TConf>
    where T: Copy
{
    fn job_loop(
        job_queue: Arc<SegQueue<JobSignal<T, TConf>>>,
        res_queue: Arc<SegQueue<JobResult<T>>>,
    ) -> () {
        loop {
            if let Some(signal) = job_queue.pop() {
                match signal {
                    JobSignal::Work(mut job) => {
                        job.out_buffer.clear();
                        for index in (job.offset)..(job.offset + job.count) {
                            job.out_buffer.push(Self::process_job(&job.buffer, index, &*job.conf));
                        }
                        res_queue.push(job.into());
                    },
                    JobSignal::Death => return,
                }
            } else {
                thread::yield_now();
            }
        }
    }

    fn process_job(buffer: &Buffer<T>, index: usize, conf: &TConf) -> T; 
}

pub trait Executor<T, TConf>
    where
        T: Clone,
        TConf: Clone,
{
    fn compute(&self, in_buffer: Buffer<T>, out_buffer: &mut [T], conf: TConf) -> Buffer<T>;

    fn compute_iterations(&self, iterations: usize, mut buffer: Buffer<T>, conf: TConf) -> Buffer<T> {
        let mut toggle = true;
        let mut buffer2 = Buffer::clone(&buffer);
        let now = Instant::now();
        for _ in 0..iterations {
            match toggle {
                true => {
                    buffer = self.compute(buffer, &mut buffer2.data, TConf::clone(&conf));
                },
                false => {
                    buffer2 = self.compute(buffer2, &mut buffer.data, TConf::clone(&conf));
                },
            };
            toggle = !toggle;
        }
        let elapsed = now.elapsed().as_millis() as f32 / 1000_f32;
        let elapsed_per_iter = elapsed / iterations as f32;
        let iter_per_sec = 1_f32 / elapsed_per_iter;
        println!("Time elapsed: {}s, {}s per iteration, {} iterations per second", elapsed, elapsed_per_iter, iter_per_sec);
        return match toggle {
            true => buffer,
            false => buffer2,
        };
    }
}

pub struct ExecutorParallel<T, TConf>
{
    job_queue: Arc<SegQueue<JobSignal<T, TConf>>>,
    res_queue: Arc<SegQueue<JobResult<T>>>,
    threads: Vec<JoinHandle<()>>,
    work_slice_len: usize,
    slices: RefCell<Vec<Vec<T>>>,
}

impl<T, TConf> ExecutorParallel<T, TConf>
    where
        T: 'static + Send + Sync + Copy,
        TConf: 'static + Send + Sync,
{
    pub fn new<TJobber: Jobber<T, TConf>>(thread_count: usize, work_slice_len: usize) -> Self {
        let thread_count = usize::max(thread_count, 1);
        let work_slice_len = usize::max(work_slice_len, 1);

        let job_queue = Arc::new(SegQueue::new());
        let res_queue = Arc::new(SegQueue::new());

        let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            let job_queue_clone = Arc::clone(&job_queue);
            let res_queue_clone = Arc::clone(&res_queue);
            threads.push(thread::spawn(move || {
                TJobber::job_loop(job_queue_clone, res_queue_clone);
            }));
        }

        Self {
            job_queue,
            res_queue,
            threads,
            work_slice_len,
            slices: RefCell::new(Vec::new()),
        }
    }

    pub fn get_slice(&self) -> Vec<T> {
        return self.slices.borrow_mut().pop().unwrap_or(Vec::with_capacity(self.work_slice_len));
    }

    pub fn push_slice(&self, slice: Vec<T>) {
        return self.slices.borrow_mut().push(slice);
    }
}

impl<T, TConf> Executor<T, TConf> for ExecutorParallel<T, TConf>
    where
        T: 'static + Send + Sync + Copy,
        TConf: 'static + Send + Sync + Clone,
{
    fn compute(&self, in_buffer: Buffer<T>, out_buffer: &mut [T], conf: TConf) -> Buffer<T> {
        let buffer_len = in_buffer.len();
        let slice_count = buffer_len / self.work_slice_len;
        let slice_leftover = buffer_len % self.work_slice_len;

        let buffer = Arc::new(in_buffer);
        let conf = Arc::from(conf);

        for i in 0..slice_count {
            let buffer_clone = Arc::clone(&buffer);
            let conf_clone = Arc::clone(&conf);
            let count = self.work_slice_len;
            let offset = i * self.work_slice_len;
            let job = JobDescriptor {
                buffer: buffer_clone,
                conf: conf_clone,
                out_buffer: self.get_slice(),
                count,
                offset,
            };
            self.job_queue.push(JobSignal::Work(job));
        }
        if slice_leftover > 0 {
            let buffer_clone = Arc::clone(&buffer);
            let conf_clone = Arc::clone(&conf);
            let count = slice_leftover;
            let offset = slice_count * self.work_slice_len;
            let job = JobDescriptor {
                buffer: buffer_clone,
                conf: conf_clone,
                out_buffer: self.get_slice(),
                count,
                offset,
            };
            self.job_queue.push(JobSignal::Work(job));
        }

        let true_slice_count = slice_count + (if slice_leftover > 0 { 1 } else { 0 });

        let mut slices: Vec<JobResult<T>> = Vec::with_capacity(true_slice_count);
        for _ in 0..true_slice_count {
            loop {
                if let Some(result) = self.res_queue.pop() {
                    slices.push(result);
                    break;
                } else {
                    thread::yield_now();
                }
            }
        }
        for slice in slices {
            out_buffer[(slice.offset)..(slice.offset + slice.count)].copy_from_slice(&slice.buffer);
            self.push_slice(slice.buffer);
        }
        return match Arc::try_unwrap(buffer) {
            Ok(buffer) => buffer,
            Err(arc) => panic!("Threaded execution error: Arc references weren't all dropped, {} remaining!", Arc::strong_count(&arc)),
        };
    }
}

impl<T, TConf> Drop for ExecutorParallel<T, TConf> {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() {
            self.job_queue.push(JobSignal::Death);
        }
        while let Some(handle) = self.threads.pop() {
            handle.join().unwrap();
        }
    }
}


pub struct ExecutorSingleThread<T, TConf, TJobber: Jobber<T, TConf>>
    where
        T: Copy,
{
    _phantom: PhantomData<(T, TConf, TJobber)>,
}

impl<T, TConf, TJobber: Jobber<T, TConf>> ExecutorSingleThread<T, TConf, TJobber>
    where
        T: Copy,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T, TConf, TJobber: Jobber<T, TConf>> Executor<T, TConf> for ExecutorSingleThread<T, TConf, TJobber>
    where
        T: Clone + Copy,
        TConf: Clone,
{
    fn compute(&self, in_buffer: Buffer<T>, out_buffer: &mut [T], conf: TConf) -> Buffer<T> {
        for index in 0..(in_buffer.len()) {
            out_buffer[index] = TJobber::process_job(&in_buffer, index, &conf);
        }
        return in_buffer;
    }
}
