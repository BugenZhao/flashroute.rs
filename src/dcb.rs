use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU8, Ordering},
};

use Ordering::{Acquire, SeqCst};

#[derive(Debug)]
pub struct DstCtrlBlock {
    pub addr: Ipv4Addr,
    pub initial_ttl: u8,
    pub accurate_distance: bool,
    pub next_backward_hop: AtomicU8,
    pub next_forward_hop: AtomicU8,
    pub forward_horizon: AtomicU8,
}

impl DstCtrlBlock {
    pub fn new(addr: Ipv4Addr, initial_ttl: u8) -> Self {
        DstCtrlBlock {
            addr,
            initial_ttl,
            accurate_distance: false,
            next_backward_hop: AtomicU8::new(initial_ttl),
            next_forward_hop: AtomicU8::new(initial_ttl + 1),
            forward_horizon: AtomicU8::new(initial_ttl),
        }
    }

    pub fn update_split_ttl(&mut self, new_ttl: u8, accurate: bool) {
        if self.accurate_distance {
            return;
        }
        self.initial_ttl = new_ttl;
        self.next_backward_hop.store(new_ttl, SeqCst);
        self.next_forward_hop.store(new_ttl + 1, SeqCst);
        self.forward_horizon.store(new_ttl, SeqCst);
        self.accurate_distance = accurate;
    }
}

impl DstCtrlBlock {
    pub fn pull_backward_task(&self) -> Option<u8> {
        let result = self.next_backward_hop.fetch_update(SeqCst, SeqCst, |x| {
            if x > 0 {
                Some(x - 1)
            } else {
                None
            }
        });
        result.ok()
    }

    pub fn pull_forward_task(&self) -> Option<u8> {
        let result = self.next_forward_hop.fetch_update(SeqCst, SeqCst, |x| {
            // TODO: more elegant way?
            if x <= self.forward_horizon.load(SeqCst) {
                Some(x + 1)
            } else {
                None
            }
        });
        result.ok()
    }

    pub fn last_forward_task(&self) -> u8 {
        let next = self.next_forward_hop.load(Acquire);
        if next == 0 {
            0
        } else {
            next - 1
        }
    }

    pub fn set_forward_horizon(&self, new_horizon: u8) {
        if new_horizon == 0 {
            return;
        }
        self.forward_horizon.fetch_max(new_horizon, SeqCst);
    }

    pub fn stop_backward(&self) -> u8 {
        self.next_backward_hop.fetch_min(0, SeqCst)
    }

    pub fn stop_forward(&self) {
        self.forward_horizon.fetch_min(0, SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    lazy_static! {
        static ref IP: Ipv4Addr = "0.0.0.0".parse().unwrap();
    }

    #[test]
    fn test_backward_task() {
        let dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_backward_task(), Some(3));
        assert_eq!(dcb.pull_backward_task(), Some(2));
        assert_eq!(dcb.pull_backward_task(), Some(1));
        assert_eq!(dcb.pull_backward_task(), None);
    }

    #[test]
    fn test_forward_task() {
        let dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_forward_task(), None);
        dcb.set_forward_horizon(5);
        assert_eq!(dcb.pull_forward_task(), Some(4));
        assert_eq!(dcb.pull_forward_task(), Some(5));
        assert_eq!(dcb.pull_forward_task(), None);
    }

    #[test]
    fn test_stop_backward_task() {
        let dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_backward_task(), Some(3));
        dcb.stop_backward();
        assert_eq!(dcb.pull_backward_task(), None);
    }

    #[test]
    fn test_stop_forward_task() {
        let dcb = DstCtrlBlock::new(*IP, 3);
        dcb.set_forward_horizon(5);
        assert_eq!(dcb.pull_forward_task(), Some(4));
        dcb.stop_forward();
        assert_eq!(dcb.pull_forward_task(), None);
    }
}
