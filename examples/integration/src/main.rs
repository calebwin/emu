extern crate em;
use em::emu;

emu! {
    // function to integrate over
    function f(x f32) f32 {
        return pow(E, pow(x, 2));
    }

    // estimate integral over given positions with given lengths between them using Midpoint method
    function integrate_f(position_indices [i32], positions [f32], lengths [f32], result [f32]) {
        let index: i32 = position_indices[..];
        result[..] += lengths[index] * f(positions[index]);
    }

    fn integrate_f(position_indices: &mut Vec<i32>, positions: &mut Vec<f32>, lengths: &mut Vec<f32>, result: &mut Vec<f32>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrate_f() {
        let mut i = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut positions = vec![0.125, 0.375, 0.625, 0.875, 1.125, 1.375, 1.625, 1.875, 2.125, 2.375];
        let mut lengths = vec![0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25];
        let mut result = vec![0.0];

        integrate_f(&mut i, &mut positions, &mut lengths, &mut result);
        assert_eq!(result, vec![109.17401]);
    }
}