extern crate em;
use em::emu;

emu! {
    // Multiplies any value in data by scalar
    // Values in buffer and scalar are 32-bit floats
    function add(data [f32], scalar f32) {
        data[..] += scalar;
    }

	// Multiplies any value in data by scalar
    // Values in buffer and scalar are 32-bit floats
    function multiply(data [f32], scalar f32) {
        data[..] *= scalar;
    }

    /// Adds each value in data to scalar
    /// Values in data and scalar are 32-bit floats
    pub fn add(data: &mut Vec<f32>, scalar: &f32);

    /// Multiplies each value in data by scalar
    /// Values in data and scalar are 32-bit floats
    pub fn multiply(data: &mut Vec<f32>, scalar: &f32);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_add() {
		let mut test_data = vec![0.5, -2.7, 9.0];
		add(&mut test_data, &1.0);
		assert_eq!(test_data, vec![1.5, -1.7, 10.0]);
	}

	#[test]
	fn test_multiply() {
		let mut test_data = vec![2.2, 2.3, 7.1];
		multiply(&mut test_data, &-2.0);
		assert_eq!(test_data, vec![-4.4, -4.6, -14.2]);
	}
}