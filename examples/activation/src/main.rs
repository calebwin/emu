extern crate em;
use em::emu;

// note: this is not intended to be a holistic library of activation functions
// it shouldn't be hard to implement most other ones so feel free to make such a library if you would like

emu! {
    // TODO implement all activation functions

    function logistic(x [f32]) {
        x[..] = 1 / (1 + pow(E, -x[..]));
    }

    function relu(x [f32]) {
        if x[..] < 0 { x[..] = 0; }
    }

    function softmax_denominator(x [f32], result [f32]) {
        result[..] += pow(E, x[..]);
    }

    function softmax(x [f32], softmax_denominator_result f32) {
        x[..] = pow(E, x[..]) / softmax_denominator_result;
    }

    function tan_h(x [f32]) {
        x[..] = tanh(x[..]);
    }

    // TODO write documentation below

    pub fn logistic(x: &mut Vec<f32>);
    pub fn relu(x: &mut Vec<f32>);
    pub fn softmax_denominator(x: &mut Vec<f32>, result: &mut Vec<f32>);
    pub fn softmax(x: &mut Vec<f32>, softmax_denominator_result: &f32);
    pub fn tan_h(x: &mut Vec<f32>);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_logistic() {
		let mut test_data = vec![0.9, 4.9, 4.8, 3.9, 1.3];
		logistic(&mut test_data);

		assert_eq!(test_data, vec![0.7109495, 0.9926085, 0.99183744, 0.98015976, 0.785835]);
	}

    #[test]
    fn test_relu() {
        let mut test_data = vec![-0.5, 0.0, 0.5, 0.55, 0.555];
        relu(&mut test_data);

        assert_eq!(test_data, vec![0.0, 0.0, 0.5, 0.55, 0.555]);
    }

    #[test]
    fn test_softmax() {
        let mut test_data = vec![0.0, 1.0, 2.0];
        let mut temp_denominator = vec![0.0];

        softmax_denominator(&mut test_data, &mut temp_denominator);
        softmax(&mut test_data, &temp_denominator[0]);
        
        assert_eq!(test_data, vec![0.09003057, 0.24472846, 0.66524094]);
    }

    #[test]
    fn test_tan_h() {
        let mut test_data = vec![9.8, 5.6, 2.9, 2.9, 1.0];
        tan_h(&mut test_data);

        assert_eq!(test_data, vec![1.0, 0.99997264, 0.9939632, 0.9939632, 0.7615942]);
    }
}