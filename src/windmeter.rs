use crate::moving_average::{self, MovingAverage};

struct Wind<const N: usize> {
    direction: i32,
    speed: f32,

    filtered_speed: f32,
    filtered_direction: i32,

    speed_buffer: moving_average::MovingAverage<N>,
}

impl<const N: usize> Wind<N> {
    pub fn new()->Self {
        Wind{direction:0, speed:0.0, filtered_speed: 0.0, filtered_direction: 0, speed_buffer: N}
    }

    pub fn update_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
    
    pub fn update_direction(&mut self, direction: i32) {
        self.direction = direction;
    }

    fn filter_speed(&mut self) {
        self.filtered_speed = self.speed;
    }
    
    fn filter_direction(&mut self) {
        self.filtered_direction = self.direction;
    }
}