use em::*;

#[gpu_use]
fn main() {
    let mut data = vec![0.1; 1000];

    gpu_do!(load(data));
    gpu_do!(launch());
    for i in 0..1000 {
        data[i] = data[i] * 10.0;
    }
    gpu_do!(read(data));

    println!("{:?}", data);
}