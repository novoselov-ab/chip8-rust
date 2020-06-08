use std::fs;

struct Memory
{
    bytes: [u8; 65536]
}

pub fn load_rom(romfile: &str)
{
    println!("Reading... {0}", romfile);

    let contents = match fs::read(romfile) {
        Err(e) => {
            println!("Can't read file: '{0}'. Error: {1}", romfile, e);
            std::process::exit(0)
        },
        Ok(f) => f
    };

    println!("Content:");
    for b in contents {
        println!("{0}", b);
    }

    let m = Memory {
        bytes: [0; 65536]
    };

    println!("Memory:");
    //println!("{0}", m.bytes);
    
    // for b in m.bytes.iter() {
    //     println!("{0}", b);
    // }
}