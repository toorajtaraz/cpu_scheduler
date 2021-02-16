use std::sync::{Arc, Mutex, MutexGuard, Barrier,Condvar};
use std::collections::VecDeque;
use std::sync::mpsc::{Sender, Receiver};
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Resource {
    A,
    B,
    C
}

#[derive(Debug, Copy, Clone)]
pub enum Algorithm{
    RR,
    SJF,
    FCFS,
    MLQ
}

#[derive(Debug, Copy, Clone)]
pub enum TaskType {
    X,
    Y,
    Z
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name : String,
    pub task_type : TaskType,
    pub resourses : (Resource, Resource),
    pub total_time : u16,
    pub time_executed : u16
}

impl Task {
    pub fn new(name: String, task_type: TaskType, time: u16) -> Task {
        match task_type {
            TaskType::X => {
                return Task{name, task_type, resourses: (Resource::A, Resource::B), total_time: time, time_executed: 0};
            },
            TaskType::Y => {
                return Task{name, task_type, resourses: (Resource::B, Resource::C), total_time: time, time_executed: 0};
            },
            TaskType::Z => {
                return Task{name, task_type, resourses: (Resource::A, Resource::C), total_time: time, time_executed: 0};
            }
        } 
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t{}: {{\n\t\ttotal time: {}\n\t\texecuted time: {}\n\t\ttime left: {}\n\t}}", self.name, self.total_time, self.time_executed, self.total_time - self.time_executed)
    }
}

pub fn sleep_core(core_pair: Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cond_var) = &*core_pair;
    let mut running = lock.lock().unwrap();
    while !*running {
        running = cond_var.wait(running).unwrap();
        if *running == true {
            break;
        }
    }
    *running = false;
}

pub fn wake_core(core_pair: Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cond_var) = &*core_pair;
    let mut running = lock.lock().unwrap();
    *running = true;
    cond_var.notify_all();
}

pub fn check_for_enough_resourse(ref mut have: &MutexGuard<((Resource, u16),(Resource, u16),(Resource, u16))>, need: (Resource, Resource)) -> bool {
    match need {
        (Resource::A, Resource::B) => {
            if (*have).0.1 == 0 || (*have).1.1 == 0 {
                return false;
            }
        },
        (Resource::A, Resource::C) => {
            if (*have).0.1 == 0 || (*have).2.1 == 0 {
                return false;
            }
        },
        (Resource::B, Resource::C) => {
            if (*have).1.1 == 0 || (*have).2.1 == 0 {
                return false;
            }
        }
        _ => {}
    }
    true
}

pub fn release_resources(have: &mut MutexGuard<((Resource, u16),(Resource, u16),(Resource, u16))>, need: &(Resource, Resource)) -> bool {
    match need {
        (Resource::A, Resource::B) => {
            (*have).0.1 += 1;
            (*have).1.1 += 1;
        },
        (Resource::A, Resource::C) => {
            (*have).0.1 += 1;
            (*have).2.1 += 1;
        },
        (Resource::B, Resource::C) => {
            (*have).1.1 += 1;
            (*have).2.1 += 1;
        }
        _ => {}
    }
    true
}

pub fn require_resources(have: &mut MutexGuard<((Resource, u16),(Resource, u16),(Resource, u16))>, need: &(Resource, Resource)) -> bool {
    match need {
        (Resource::A, Resource::B) => {
            (*have).0.1 -= 1;
            (*have).1.1 -= 1;
        },
        (Resource::A, Resource::C) => {
            (*have).0.1 -= 1;
            (*have).2.1 -= 1;
        },
        (Resource::B, Resource::C) => {
            (*have).1.1 -= 1;
            (*have).2.1 -= 1;
        }
        _ => {}
    }
    true
}

pub fn cpu_worker(tx: Sender<String>, id: String, core_pair: Arc<(Mutex<bool>, Condvar)>, queue: Arc<Mutex<VecDeque<Task>>>, w_queue: Arc<Mutex<VecDeque<Task>>>, resourses: Arc<Mutex<((Resource, u16),(Resource, u16),(Resource, u16))>>, algo: Arc<Algorithm>, master: Arc<Barrier>, queues: Arc<Mutex<Vec<VecDeque<Task>>>>) {
    match *algo {
        Algorithm::FCFS => {
            let mut on_proc = false;
            let mut proc = None;
            let mut idle_count = 0;
            loop {
                let ref mut proc = proc;
                sleep_core(core_pair.clone());
                if !on_proc {
                    let lock  = &*(queue).clone();
                    let mut q = lock.lock().unwrap();
                    let temp = (*q).pop_front();
                    *proc = temp;
                    match proc {
                        Some(p) => {
                            let lock = &*(resourses);
                            let mut r = lock.lock().unwrap();
                            if !check_for_enough_resourse(&r, p.resourses) {
                                let w_lock =  &*(w_queue).clone();
                                let mut wq = w_lock.lock().unwrap();
                                (*wq).push_back((*p).clone());
                                *proc = None;
                            } else {
                                on_proc = true;
                                require_resources(&mut r, &p.resourses);
                            }
                        },
                        None => {}
                    }
                }
                match proc {
                    Some(p) => {
                        p.time_executed = p.time_executed + 1;
                        tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                        if p.total_time - p.time_executed == 0 {
                            {
                                let lock = &*(resourses);
                                let mut r = lock.lock().unwrap();
                                release_resources(&mut r, &p.resourses);
                            }
                            *proc = None;
                            on_proc = false;
                        }
                    },
                    None => {
                        idle_count += 1;
                        on_proc = false;
                        tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                    }
                }
                master.wait();
            } 
        },
        Algorithm::SJF => {
            let mut on_proc = false;
            let mut proc = None;
            let mut idle_count = 0;
            loop {
                let ref mut proc = proc;
                sleep_core(core_pair.clone());
                if !on_proc {
                    let lock  = &*(queue).clone();
                    let mut q = lock.lock().unwrap();
                    let temp = (*q).pop_front();
                    *proc = temp;
                    match proc {
                        Some(p) => {
                            let lock = &*(resourses);
                            let mut r = lock.lock().unwrap();
                            if !check_for_enough_resourse(&r, p.resourses) {
                                let w_lock =  &*(w_queue).clone();
                                let mut wq = w_lock.lock().unwrap();
                                (*wq).push_back((*p).clone());
                                (*wq).make_contiguous().sort_by(|a, b| a.total_time.cmp(&b.total_time));
                                *proc = None;
                            } else {
                                on_proc = true;
                                require_resources(&mut r, &p.resourses);
                            }
                        },
                        None => {}
                    }
                }
                match proc {
                    Some(p) => {
                        p.time_executed = p.time_executed + 1;
                        tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                        if p.total_time - p.time_executed == 0 {
                            let lock = &*(resourses);
                            let mut r = lock.lock().unwrap();
                            release_resources(&mut r, &p.resourses); 
                            *proc = None;
                            on_proc = false;
                        }
                    },
                    None => {
                        on_proc = false;
                        idle_count += 1;
                        tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                    }
                }
                master.wait();
            } 
        },
        Algorithm::RR => {
            let mut proc = None;
            let mut idle_count = 0;
            loop {
                let ref mut proc = proc;
                sleep_core(core_pair.clone());
                loop {
                    let lock  = &*(queue).clone();
                    let mut q = lock.lock().unwrap();
                    let lock = &*(resourses);
                    let r = lock.lock().unwrap();
                    let temp = (*q).pop_front();
                    *proc = temp;
                    match proc {
                        Some(p) => {
                            if !check_for_enough_resourse(&r, p.resourses) {
                                let w_lock =  &*(w_queue).clone();
                                let mut wq = w_lock.lock().unwrap();
                                (*wq).push_back((*p).clone());
                            } else {
                                break;
                            }
                        },
                        None => {
                            break;
                        }
                    }
                }
                match proc {
                    Some(p) => {
                        let lock  = &*(queue).clone();
                        let mut q = lock.lock().unwrap();
                        p.time_executed = p.time_executed + 1;
                        if p.total_time - p.time_executed > 0 {
                            (*q).push_back((*p).clone());
                        }
                        tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                    },
                    None => {
                        idle_count += 1;
                        tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                    }
                }   
                master.wait();
            }
        },
        Algorithm::MLQ => {
            let mut idle_count = 0;
            let mut on_proc = false;
            let mut proc : Option<Task>= None;

            loop {
                sleep_core(core_pair.clone());
                let lock  = &*(queues).clone();
                let mut qs = lock.lock().unwrap();
                if qs[2].len() > 0 {
                    match proc {
                        Some(ref mut p) => {
                            qs[0].push_front(p.clone());
                            proc = None;
                        },
                        _ => ()
                    }
                    let ref mut q = qs[2];
                    let ref mut proc = proc;
                    loop {
                        let lock = &*(resourses);
                        let r = lock.lock().unwrap();
                        let temp = q.pop_front();
                        *proc = temp;
                        match proc {
                            Some(p) => {
                                if !check_for_enough_resourse(&r, p.resourses) {
                                    let w_lock =  &*(w_queue).clone();
                                    let mut wq = w_lock.lock().unwrap();
                                    (*wq).push_back((*p).clone());
                                } else {
                                    break;
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    }
                    match proc {
                        Some(p) => {
                            p.time_executed = p.time_executed + 1;
                            if p.total_time - p.time_executed > 0 {
                                q.push_back((*p).clone());
                            }
                            tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                        },
                        None => {
                            idle_count += 1;
                            tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                        }
                    }   
                    *proc = None;
                }
                else if qs[1].len() > 0 {
                    match proc {
                        Some(ref mut p) => {
                            qs[0].push_front(p.clone());
                            proc = None;
                        },
                        _ => ()
                    }
                    let ref mut q = qs[1];
                    let ref mut proc = proc;
                    loop {
                        let lock = &*(resourses);
                        let r = lock.lock().unwrap();
                        let temp = q.pop_front();
                        *proc = temp;
                        match proc {
                            Some(p) => {
                                if !check_for_enough_resourse(&r, p.resourses) {
                                    let w_lock =  &*(w_queue).clone();
                                    let mut wq = w_lock.lock().unwrap();
                                    (*wq).push_back((*p).clone());
                                } else {
                                    break;
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    }
                    match proc {
                        Some(p) => {
                            p.time_executed = p.time_executed + 1;
                            if p.total_time - p.time_executed > 0 {
                                q.push_back((*p).clone());
                            }
                            tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                        },
                        None => {
                            idle_count += 1;
                            tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                        }
                    }  
                    *proc = None;
                }
                else {
                    let ref mut q = qs[0]; 
                    let ref mut proc = proc;
                    if !on_proc {
                        let temp = (*q).pop_front();
                        *proc = temp;
                        match proc {
                            Some(p) => {
                                let lock = &*(resourses);
                                let mut r = lock.lock().unwrap();
                                if !check_for_enough_resourse(&r, p.resourses) {
                                    let w_lock =  &*(w_queue).clone();
                                    let mut wq = w_lock.lock().unwrap();
                                    (*wq).push_back((*p).clone());
                                    *proc = None;
                                } else {
                                    on_proc = true;
                                    require_resources(&mut r, &p.resourses);
                                }
                            },
                            None => {}
                        }
                    }
                    match proc {
                        Some(p) => {
                            p.time_executed = p.time_executed + 1;
                            tx.send(std::format!("{} is processing:\n{}", id, p)).unwrap();
                            if p.total_time - p.time_executed == 0 {
                                {
                                    let lock = &*(resourses);
                                    let mut r = lock.lock().unwrap();
                                    release_resources(&mut r, &p.resourses);
                                }
                                *proc = None;
                                on_proc = false;
                            }
                        },
                        None => {
                            idle_count += 1;
                            on_proc = false;
                            tx.send(std::format!("core {} idle, idle count: {}", id, idle_count)).unwrap();
                        }
                    }
                }
                drop(lock);
                drop(qs);
                master.wait();
            }
        }
    }
}

pub fn master_worker(rx: Receiver<String>, cores: Vec<Arc<(Mutex<bool>, Condvar)>>, resourses: Arc<Mutex<((Resource, u16),(Resource, u16),(Resource, u16))>>, queue: Arc<Mutex<VecDeque<Task>>>, w_queue: Arc<Mutex<VecDeque<Task>>>, algo: Arc<Algorithm>, master: Arc<Barrier>, queues: Arc<Mutex<Vec<VecDeque<Task>>>>) {
     {
        let lock  = &*(queue).clone();
        let q = lock.lock().unwrap();
        println!("QUEUE: [");
        for i in 0..(*q).len() {
            println!("{}", &(*q)[i]);
        }
        println!("]");
    }
    let mut clocks = 0;
    loop {
        clocks += 1;
        let rx = &rx;
        for i in 0..4 {
            wake_core(cores[i].clone());
        }
        master.wait();
        match *algo {
            Algorithm::MLQ => { 
                let lock  = &*(queues).clone();
                let mut qs = lock.lock().unwrap();
                let r_lock = &*(resourses);
                let r = r_lock.lock().unwrap();
                println!("<<at {} clock>>", clocks);
                println!("Resources : {:?}", r); 
                println!("Z QUEUE: [");
                for i in 0..(qs.get_mut(2).unwrap()).len() {
                    println!("{}", &(qs.get_mut(2).unwrap())[i]);
                }
                println!("]");
                println!("Y QUEUE: [");
                for i in 0..(qs.get_mut(1).unwrap()).len() {
                    println!("{}", &(qs.get_mut(1).unwrap())[i]);
                }
                println!("]");
                println!("X QUEUE: [");
                for i in 0..(qs.get_mut(0).unwrap()).len() {
                    println!("{}", &(qs.get_mut(0).unwrap())[i]);
                }
                println!("]");
                let mut idle_count = 0;
                for _ in 0..4 {
                    let msg = rx.recv().unwrap();
                    if msg.contains("idle") {
                        idle_count += 1;
                    }
                    println!("{}", msg);
                }
                let w_lock =  &*(w_queue).clone();
                let mut wq = w_lock.lock().unwrap();
                let temp = (*wq).pop_front();
                match temp {
                    Some(p) => {
                        if check_for_enough_resourse(&r, p.resourses) {
                            match p.task_type {
                                TaskType::X => {
                                    (qs.get_mut(0).unwrap()).push_back(p);
                                },
                                TaskType::Y => {
                                    (qs.get_mut(1).unwrap()).push_back(p);
                                },
                                TaskType::Z => {
                                    (qs.get_mut(2).unwrap()).push_back(p);
                                }
                            }

                        } else {
                            (*wq).push_back(p);
                        }
                    },
                    None => {}
                }
                if idle_count == 4 && (qs.get_mut(0).unwrap()).len() == 0 && (qs.get_mut(1).unwrap()).len() == 0 && (qs.get_mut(2).unwrap()).len() == 0 && (*wq).len() == 0 {
                    println!("TOTAL CLOCKS:  {}", clocks);
                    return;
                } 
            },
            _ => {
                let lock  = &*(queue).clone();
                let mut q = lock.lock().unwrap();
                let r_lock = &*(resourses);
                let r = r_lock.lock().unwrap();
                println!("<<at {} clock>>", clocks);
                println!("Resources : {:?}", r); 
                println!("QUEUE: [");
                for i in 0..(*q).len() {
                    println!("{}", &(*q)[i]);
                }
                println!("]");
                let mut idle_count = 0;
                for _ in 0..4 {
                    let msg = rx.recv().unwrap();
                    if msg.contains("idle") {
                        idle_count += 1;
                    }
                    println!("{}", msg);
                }
                let w_lock =  &*(w_queue).clone();
                let mut wq = w_lock.lock().unwrap();
                let temp = (*wq).pop_front();
                match temp {
                    Some(p) => {
                        if check_for_enough_resourse(&r, p.resourses) {
                            (*q).push_back(p);
                            match *algo {
                                Algorithm::SJF => {
                                    (*q).make_contiguous().sort_by(|a, b| a.total_time.cmp(&b.total_time)); 
                                },
                                _ => {}
                            }
                        } else {
                            (*wq).push_back(p);
                            match *algo {
                                Algorithm::SJF => {
                                    (*wq).make_contiguous().sort_by(|a, b| a.total_time.cmp(&b.total_time)); 
                                },
                                _ => {}
                            }
                        }
                    },
                    None => {}
                }
                if idle_count == 4 && (*q).len() == 0 && (*wq).len() == 0 {
                    println!("TOTAL CLOCKS:  {}", clocks);
                    return;
                }
            }
            }
        }
}
