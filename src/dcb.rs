use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU8, Ordering},
};

pub struct DstCtrlBlock {
    addr: Ipv4Addr,
    initial_ttl: u8,
    accurate_distance: bool,
    next_backward_hop: AtomicU8,
    next_forward_hop: AtomicU8,
    forward_horizon: AtomicU8,
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
        self.next_backward_hop.store(new_ttl, Ordering::SeqCst);
        self.next_forward_hop.store(new_ttl + 1, Ordering::SeqCst);
        self.forward_horizon.store(new_ttl, Ordering::SeqCst);
        self.accurate_distance = accurate;
    }
}

impl DstCtrlBlock {
    pub fn pull_backward_task(&mut self) -> Option<u8> {
        let result = self
            .next_backward_hop
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                if x > 0 {
                    Some(x - 1)
                } else {
                    None
                }
            });
        result.ok()
    }

    pub fn pull_forward_task(&mut self) -> Option<u8> {
        let result = self
            .next_forward_hop
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                // TODO: more elegant way?
                if x <= self.forward_horizon.load(Ordering::SeqCst) {
                    Some(x + 1)
                } else {
                    None
                }
            });
        result.ok()
    }

    pub fn set_forward_horizon(&mut self, new_horizon: u8) {
        if new_horizon == 0 {
            return;
        }
        self.forward_horizon
            .fetch_max(new_horizon, Ordering::SeqCst);
    }

    pub fn stop_backward(&mut self) -> u8 {
        self.next_backward_hop.fetch_min(0, Ordering::SeqCst)
    }

    pub fn stop_forward(&mut self) {
        self.forward_horizon.fetch_min(0, Ordering::SeqCst);
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
        let mut dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_backward_task(), Some(3));
        assert_eq!(dcb.pull_backward_task(), Some(2));
        assert_eq!(dcb.pull_backward_task(), Some(1));
        assert_eq!(dcb.pull_backward_task(), None);
    }

    #[test]
    fn test_forward_task() {
        let mut dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_forward_task(), None);
        dcb.set_forward_horizon(5);
        assert_eq!(dcb.pull_forward_task(), Some(4));
        assert_eq!(dcb.pull_forward_task(), Some(5));
        assert_eq!(dcb.pull_forward_task(), None);
    }

    #[test]
    fn test_stop_backward_task() {
        let mut dcb = DstCtrlBlock::new(*IP, 3);
        assert_eq!(dcb.pull_backward_task(), Some(3));
        dcb.stop_backward();
        assert_eq!(dcb.pull_backward_task(), None);
    }

    #[test]
    fn test_stop_forward_task() {
        let mut dcb = DstCtrlBlock::new(*IP, 3);
        dcb.set_forward_horizon(5);
        assert_eq!(dcb.pull_forward_task(), Some(4));
        dcb.stop_forward();
        assert_eq!(dcb.pull_forward_task(), None);
    }
}
