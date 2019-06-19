extern crate em;
use em::emu;

emu! {
    function sum(x [f32], result [f32]) {
        result[..] += x[..];
    }

    pub fn sum(x: &mut Vec<f32>, result: &mut Vec<f32>);
}

pub fn average(x: &mut Vec<f32>) -> f32 {
    let mut summation = vec![0.0];

    sum(x, &mut summation);

    summation[0] / x.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let mut x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let mut summation = vec![0.0];

        sum(&mut x, &mut summation);
        assert_eq!(summation, vec![45.0]);
    }

    #[test]
    fn test_average() {
        let mut x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        assert_eq!(average(&mut x), 4.5);
    }
}