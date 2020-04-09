# CMake generated Testfile for 
# Source directory: /home/caleb/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.6.2/build/shaderc/glslc/test
# Build directory: /home/caleb/Projects/emu/docs/debug/build/shaderc-sys-cef2278ad5b6e469/out/build/shaderc/glslc/test
# 
# This file includes the relevant testing commands required for 
# testing this directory and lists subdirectories to be tested as well.
add_test(shaderc_expect_unittests "/usr/bin/python3" "-m" "unittest" "expect_unittest.py")
set_tests_properties(shaderc_expect_unittests PROPERTIES  WORKING_DIRECTORY "/home/caleb/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.6.2/build/shaderc/glslc/test")
add_test(shaderc_glslc_test_framework_unittests "/usr/bin/python3" "-m" "unittest" "glslc_test_framework_unittest.py")
set_tests_properties(shaderc_glslc_test_framework_unittests PROPERTIES  WORKING_DIRECTORY "/home/caleb/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.6.2/build/shaderc/glslc/test")
