use libc::{
    mq_attr, mq_close, mq_open, mq_receive, mq_timedreceive, mq_unlink, mqd_t, O_CREAT, O_RDWR,
};
use log::{info, error};

use std::cell::Cell;
use std::ffi::CString;
use std::rc::Rc;

pub mod apps;
pub mod arch_utils;
pub mod components;
pub mod configurer;

pub struct MessageQueue {
    name: CString,
    queue: mqd_t,
    attr: mq_attr,
    instance_count: Rc<Cell<usize>>,
}

impl MessageQueue {
    pub fn new(name: &str, msg_size: i64) -> Result<MessageQueue, &str> {
        let queue_name = CString::new(name).unwrap();

        unsafe { mq_unlink(queue_name.as_ptr() as *const libc::c_char) };

        let mut attr: libc::mq_attr = unsafe { std::mem::zeroed() };
        attr.mq_flags = 0;
        attr.mq_maxmsg = 1024;
        attr.mq_msgsize = msg_size;
        attr.mq_curmsgs = 0;
        let mqd = unsafe {
            mq_open(
                queue_name.as_ptr() as *const libc::c_char,
                O_CREAT | O_RDWR,
                0644,
                &attr,
            )
        };
        if mqd == -1 {
            let error_msg = format!(
                "Could not open message queue '{}'. Error: {}",
                name,
                std::io::Error::last_os_error()
            );
            error!("{}", error_msg);
            return Err("Could not open message queue. Try printing libc error.");
        }

        let mq = MessageQueue {
            name: queue_name,
            queue: mqd,
            attr,
            instance_count: Rc::new(Cell::new(1)),
        };

        mq.clear_queue();
        info!("Successfully created message queue '{}'", name);
        Ok(mq)
    }

    pub fn read_message(&self, buffer: &mut [u8]) -> Result<isize, isize> {
        let ret = unsafe {
            let mut wait_time: libc::timespec = std::mem::zeroed();
            libc::clock_gettime(libc::CLOCK_REALTIME, &mut wait_time);
            wait_time.tv_sec += 1;
    
            mq_timedreceive(
                self.queue,
                buffer.as_mut_ptr() as *mut i8,
                self.attr.mq_msgsize as usize,
                std::ptr::null_mut::<u32>(),
                &wait_time,
            )
        };
    
        if ret == -1 {
            let os_error = std::io::Error::last_os_error(); // Get the specific OS error
            error!(
                "Could not read message from queue {:?}: {}",
                self.name.as_c_str(),
                os_error
            );
            return Err(ret);
        }
        Ok(ret)
    }
    

    pub fn read_message_without_timeout(&self, buffer: &mut [u8]) -> Result<isize, isize> {
        let ret = unsafe {
            mq_receive(
                self.queue,
                buffer.as_mut_ptr() as *mut i8,
                self.attr.mq_msgsize as usize,
                std::ptr::null_mut::<u32>(),
            )
        };
        if ret == -1 {
            error!(
                "Could not read message from queue {:?}",
                self.name.as_c_str()
            );
            return Err(ret);
        }
        Ok(ret)
    }

    pub fn clear_queue(&self) -> usize {
        let mut nr_messages = 0;
        let mut dummy: Vec<u8> = vec![0; self.attr.mq_msgsize as usize];
        while self.read_message(&mut dummy).is_ok() {
            nr_messages += 1;
        }
        nr_messages
    }
}

impl Clone for MessageQueue {
    fn clone(&self) -> MessageQueue {
        let new_count = self.instance_count.get() + 1;
        self.instance_count.set(new_count);
        MessageQueue {
            name: self.name.clone(),
            queue: self.queue,
            attr: self.attr,
            instance_count: self.instance_count.clone(),
        }
    }
}

impl std::ops::Drop for MessageQueue {
    fn drop(&mut self) {
        self.instance_count.set(self.instance_count.get() - 1);
        if self.instance_count.get() == 0 {
            unsafe {
                if mq_close(self.queue) != 0 {
                    let message = CString::new("Closing mqd").unwrap();
                    libc::perror(message.as_ptr() as *const i8);
                };
                if mq_unlink(self.name.as_ptr() as *const libc::c_char) != 0 {
                    let message = CString::new("unlinking mq").unwrap();
                    libc::perror(message.as_ptr() as *const i8);
                }
            }
        }
    }
}

pub struct Average {
    pub nr_values: u64,
    pub average: f64,
}

impl Average {
    pub fn new() -> Average {
        Average {
            nr_values: 0,
            average: 0.0,
        }
    }

    pub fn update(&mut self, value: f64) {
        self.nr_values += 1;

        let nr_vals = self.nr_values as f64;
        self.average = ((nr_vals - 1.0) / nr_vals) * self.average + (1.0 / nr_vals) * value;
    }

    pub fn reset(&mut self) {
        self.nr_values = 0;
        self.average = 0.0;
    }
}
