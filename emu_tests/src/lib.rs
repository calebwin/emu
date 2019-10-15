// use em::*;

#[cfg(test)]
mod tests {
    use em::*;

    // this tests that we can detect when the #[gpu_use] macro
    // is used incorrectly
    fn test_macro_usage(t: &trybuild::TestCases) {
        t.compile_fail("src/macro_usage_0.rs");
        t.pass("src/macro_usage_1.rs");
        t.compile_fail("src/macro_usage_2.rs");
        t.pass("src/macro_usage_3.rs");
        t.compile_fail("src/macro_usage_4.rs");
        t.compile_fail("src/macro_usage_5.rs");
        t.compile_fail("src/macro_usage_6.rs");
        t.compile_fail("src/macro_usage_7.rs");
        t.compile_fail("src/macro_usage_8.rs");
        t.pass("src/macro_usage_9.rs");
        t.pass("src/macro_usage_10.rs");
        t.pass("src/macro_usage_11.rs");
    }

    // this tests that bad usage of load and read macro are detected
    fn test_load_read(t: &trybuild::TestCases) {
        t.compile_fail("src/load_read_0.rs");
        t.compile_fail("src/load_read_1.rs");
        t.pass("src/load_read_2.rs");
        t.compile_fail("src/load_read_3.rs");
        t.compile_fail("src/load_read_4.rs");
    }

    // this tests that bad usage of launch (like launching things
    // that can't be launched) is detected
    fn test_launch(t: &trybuild::TestCases) {
        t.compile_fail("src/launch_0.rs");
        t.compile_fail("src/launch_1.rs");
        t.compile_fail("src/launch_2.rs");
        t.compile_fail("src/launch_3.rs");
        t.compile_fail("src/launch_4.rs");
        t.compile_fail("src/launch_5.rs");
        t.pass("src/launch_6.rs");
    }

    // test the compile-time errors
    #[test]
    #[gpu_use(test_all_panics)]
    fn test_all() {
        let t = trybuild::TestCases::new();

        // test that things get compiled correctly
        test_macro_usage(&t);
        test_load_read(&t);
        test_launch(&t);
    }

    // test for run-time errors
    #[test]
    #[gpu_use]
    fn test_all_panics() {
        let mut data = vec![1.0; 1000];
        gpu_do!(load(data));
        gpu_do!(launch());
        for i in 0..1000 {
            data[i] = data[i] * 10.0;
        }
        assert_eq!(data, vec![1.0; 1000]);
        gpu_do!(read(data));
        assert_eq!(data, vec![10.0; 1000]);
        gpu_do!(launch());
        for i in 0..1000 {
            data[i] = data[i] * 10.0;
        }
        assert_eq!(data, vec![10.0; 1000]);
        gpu_do!(read(data));
        assert_eq!(data, vec![100.0; 1000]);
    }

    #[test]
    #[gpu_use]
    #[should_panic(expected = "not loaded to GPU")]
    fn test_panic_what_0() {
        let mut data = vec![1.0; 1000];
        gpu_do!(launch());
        for i in 0..1000 {
            data[i] = data[i] * 10.0;
        }
        gpu_do!(read(data));
    }

    #[test]
    #[gpu_use]
    #[should_panic(expected = "cannot be empty")]
    fn test_panic_what_1() {
        let data = vec![1.0; 0];
        gpu_do!(load(data));
    }
}
