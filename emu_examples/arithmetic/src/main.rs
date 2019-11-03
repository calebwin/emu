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

// fn main() {
// 	// i = get_group_id
// 	// j = get_local_id

// 	for (i, j) in x.enumerate().map(|(i, _)| (i / 10, i % 10)) {

// 	}

// 	for (i, chunk) in x.chunks(10).enumerate() {
// 		let mut scratch = vec![0.0; 10];
// 		for (j, _) in chunk.enumerate() {

// 		}
// 	}

// }
