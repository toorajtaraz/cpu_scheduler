mod util;
use util::workers::*;
use std::sync::{Arc, Mutex, Barrier,Condvar};
use std::thread;
use std::collections::VecDeque;
use std::sync::mpsc::channel;
use std::io;
fn main() {
    println!("FOR FCFS 1\nFOR SJF 2\nFOR RR 3\nFOR MLQ 4");
    let mut algo = Arc::new(Algorithm::SJF);
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            input.remove(n - 1);
            match input.parse::<u16>().unwrap() {
                1 => {
                   algo = Arc::new(Algorithm::FCFS); 
                },
                2 => {
                   algo = Arc::new(Algorithm::SJF);
                },
                3 => {
                   algo = Arc::new(Algorithm::RR);
                },
                4 => {
                   algo = Arc::new(Algorithm::MLQ);
                },
                _ => panic!("WRONG INPUT!!")
            }
        }
        Err(error) => println!("error: {}", error),
    }
    let mut input = String::new();
    let mut resourses = (0, 0, 0);
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            input.remove(n - 1);
            let mut i = 0;
            for num in input.split(" ") {
                if i == 0 {
                    resourses.0 = num.parse::<u16>().unwrap();
                }
                if i == 1 {
                    resourses.1 = num.parse::<u16>().unwrap();
                }
                if i == 2 {
                    resourses.2 = num.parse::<u16>().unwrap();
                    break;
                }
                i += 1;
            }
        }
        Err(error) => println!("error: {}", error),
    }
    let mut input = String::new();
    let mut number_of_proc = 0;
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            input.remove(n - 1);
            number_of_proc = input.parse::<u16>().unwrap();

        }
        Err(error) => println!("error: {}", error),
    }
    let (tx, rx) = channel();
    let resourses = Arc::new(Mutex::new(((Resource::A, resourses.0), (Resource::B, resourses.1), (Resource::C, resourses.2))));
    let ready_queue = Arc::new(Mutex::new(VecDeque::<Task>::new()));
    let ready_queues = Arc::new(Mutex::new(Vec::<VecDeque<Task>>::new()));
    let mut x_q = VecDeque::<Task>::new();
    let mut y_q = VecDeque::<Task>::new();
    let mut z_q = VecDeque::<Task>::new();
    for _ in 0..number_of_proc {
        let mut name = String::new();
        let mut task_type = TaskType::X;
        let mut tt = 0;
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                    input.remove(n - 1);
                    let mut i = 0;
                    for data in input.split(" ") {
                        if i == 0 {
                           name.push_str(data.as_ref());
                        }
                        if i == 1 {
                            if data.eq("X") {
                                task_type = TaskType::X;
                            } else if data.eq("Y") {
                                task_type = TaskType::Y;
                            } else if data.eq("Z") {
                                task_type = TaskType::Z;
                            } else {
                                panic!("WRONG INPUT!");
                            }
                        }
                        if i == 2 {
                            tt = data.parse::<u16>().unwrap();
                            break;
                        }
                        i += 1;
                    }

            }
            Err(error) => println!("error: {}", error),
        }
        let lock = &*ready_queue;
        let mut q = lock.lock().unwrap();
        match *algo {
            Algorithm::MLQ => {
               match task_type {
                    TaskType::X => {
                        x_q.push_back(Task::new(name, task_type, tt));
                    },
                    TaskType::Y => {
                        y_q.push_back(Task::new(name, task_type, tt));
                    },
                    TaskType::Z => {
                        z_q.push_back(Task::new(name, task_type, tt));
                    }
               } 
            },
            _ => {
                (*q).push_back(Task::new(name, task_type, tt));
                match *algo {
                    Algorithm::SJF => {
                        (*q).make_contiguous().sort_by(|a, b| a.total_time.cmp(&b.total_time)); 
                        println!("{:?}", q);
                    },
                    _ => {}
                }
            }
        }
    }
    {
        let lock = &*ready_queues;
        let mut q = lock.lock().unwrap();
        (*q).push(x_q);
        (*q).push(y_q);
        (*q).push(z_q);
    }
    let waiting_queue = Arc::new(Mutex::new(VecDeque::<Task>::new()));
    let cores = vec![Arc::new((Mutex::new(false), Condvar::new())), Arc::new((Mutex::new(false), Condvar::new())), Arc::new((Mutex::new(false), Condvar::new())), Arc::new((Mutex::new(false), Condvar::new()))];
    let barrier = Arc::new(Barrier::new(5)); 
    for i in 0..4 {
        let barrier = barrier.clone();
        let tx = tx.clone();
        let core_clone = cores[i].clone();
        let q = Arc::clone(&ready_queue);
        let qs = Arc::clone(&ready_queues);
        let w = Arc::clone(&waiting_queue);
        let r = Arc::clone(&resourses);
        let a = Arc::clone(&algo);
        let num = i;
        thread::spawn(move|| {
            cpu_worker(tx, format!("core{}", num), core_clone, q, w, r, a, barrier, qs);
        });
    }
    master_worker(rx, cores.clone(), resourses.clone(), ready_queue.clone(), waiting_queue.clone(), algo.clone(), barrier.clone(), ready_queues.clone());
}

