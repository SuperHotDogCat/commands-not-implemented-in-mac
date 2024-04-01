use regex::Regex;
use std::env;
use std::process::Command;

struct UnixMemInfo {
    // MemInfo like Unix
    total: u64,
    used: u64,
    free: u64,
    avail: u64,
    buff_cache: u64,
    swap_total: u64,
    swap_used: u64,
    swap_free: u64,
}

fn init_unixmeminfo() -> UnixMemInfo {
    let [total, used, free, avail, buff_cache, swap_total, swap_used, swap_free] =
        extract_unixmeminfo();
    UnixMemInfo {
        total: total,
        used: used,
        free: free,
        avail: avail,
        buff_cache: buff_cache,
        swap_total: swap_total,
        swap_used: swap_used,
        swap_free: swap_free,
    }
}

fn extract_unixmeminfo() -> [u64; 8] {
    let total = extract_memsize();
    let [free, avail, buff_cache, used] = extract_ex_meminfo();
    let [swap_total, swap_used, swap_free] = extract_swapinfo();
    [
        total, used, free, avail, buff_cache, swap_total, swap_used, swap_free,
    ]
}

fn extract_memsize() -> u64 {
    let output = Command::new("sysctl")
        .arg("hw.memsize")
        .output()
        .expect("failed to execute sysctl");
    let output_str = String::from_utf8(output.stdout).unwrap();
    let re = Regex::new(r"\d+").unwrap();
    let mut memsize: u64 = re.find(&output_str).unwrap().as_str().parse().unwrap();
    memsize = memsize >> 10;
    memsize
}

fn extract_swapinfo() -> [u64; 3] {
    // sysctl
    let output = Command::new("sysctl")
        .arg("vm.swapusage")
        .output()
        .expect("failed to execute sysctl");
    let output_str = String::from_utf8_lossy(&output.stdout);
    // Define regex to extract u64 numbers.
    let re = Regex::new(r"[-+]?\d*\.\d+|\d+").unwrap();
    let extracted_num: Vec<f64> = re
        .find_iter(&output_str) // 正規表現にマッチする部分をイテレート
        .filter_map(|m| m.as_str().parse().ok()) // マッチした部分を数値に変換し、Someで返す
        .collect();

    let mut kilobytes_num: [u64; 3] = [0; 3];
    for (idx, num) in extracted_num.iter().enumerate() {
        kilobytes_num[idx] = (*num * 1024.0) as u64;
    }
    kilobytes_num
}

fn extract_ex_meminfo() -> [u64; 4] {
    // vm_stat
    let output = Command::new("vm_stat")
        .output()
        .expect("failed to execute vm_stat");
    // transform into str
    let output_str = String::from_utf8_lossy(&output.stdout);

    let lines = output_str.lines();

    // Define regex to extract u64 numbers.
    let re = Regex::new(r"\d+").unwrap();
    let mut pages_free = 0_u64;
    let mut pages_avail = 0_u64;
    let mut page_size = 0_u64;
    let mut page_cache = 0_u64;
    let mut used = 0_u64;
    let used_memory_columns = [
        "Pages wired down",
        "Pages active",
        "Pages inactive",
        "Pages speculative",
        "Pages occupied by compressor",
    ];
    for (idx, line) in lines.enumerate() {
        if idx == 0 {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            page_size = extracted_num;
        }
        if line.contains("Pages purgeable") || line.contains("File-backed pages") {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            page_cache += extracted_num * page_size;
        }
        for column_name in used_memory_columns {
            if is_used_memory(line, column_name) {
                let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
                used += extracted_num * page_size;
            }
        }

        // free and avail caliculation is as follows
        if line.contains("Pages free") {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            pages_free += extracted_num * page_size;
            pages_avail += extracted_num * page_size;
        }
        if line.contains("Pages inactive") {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            pages_avail += extracted_num * page_size;
        }
        if line.contains("Pages speculative") {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            pages_free -= extracted_num * page_size;
        }
    }
    used -= page_cache; // substract page_cache to calculate truly used memories
    page_cache = page_cache >> 10; // bytes -> kilo bytes
    used = used >> 10;
    pages_free = pages_free >> 10;
    pages_avail = pages_avail >> 10;
    [pages_free, pages_avail, page_cache, used]
}

fn is_used_memory(line: &str, column_name: &str) -> bool {
    if line.contains(column_name) {
        return true;
    }
    false
}

fn free(args: Vec<String>) {
    format_mem_info(args);
}

fn make_args() -> Vec<String> {
    //Implement -k, -m, -g, -h options
    let args: Vec<String> = env::args().collect();
    args
}

fn format_mem_info(args: Vec<String>) {
    // header, not swaped, swapedの順に出力する
    let mut header = "               total         used       free       avail   buff/cache";
    let mut unixmem_info: UnixMemInfo = init_unixmeminfo(); // This variable is mutable because options can change the value.
    let valid_options = [
        "-k".to_string(),
        "-m".to_string(),
        "-g".to_string(),
        "-h".to_string(),
    ];
    let mut memory_content = format!(
        "Mem:        {}     {}     {}     {}      {}",
        unixmem_info.total,
        unixmem_info.used,
        unixmem_info.free,
        unixmem_info.avail,
        unixmem_info.buff_cache
    );
    let mut swaped_content = format!(
        "Swap:       {}     {}     {}",
        unixmem_info.swap_total, unixmem_info.swap_used, unixmem_info.swap_free
    );
    if args.len() > 1 {
        transform_meminfo_by_option(&mut unixmem_info, &args[1]);
        if args[1] == valid_options[1] {
            header = "            total      used    free    avail buff/cache";
            memory_content = format!(
                "Mem:        {}     {}     {}     {}       {}",
                unixmem_info.total,
                unixmem_info.used,
                unixmem_info.free,
                unixmem_info.avail,
                unixmem_info.buff_cache
            );
            swaped_content = format!(
                "Swap:       {}     {}     {}",
                unixmem_info.swap_total, unixmem_info.swap_used, unixmem_info.swap_free
            );
        } else if args[1] == valid_options[2] {
            header = "         total   used  free avail buff/cache";
            memory_content = format!(
                "Mem:        {}     {}     {}     {}       {}",
                unixmem_info.total,
                unixmem_info.used,
                unixmem_info.free,
                unixmem_info.avail,
                unixmem_info.buff_cache
            );
            swaped_content = format!(
                "Swap:       {}     {}     {}",
                unixmem_info.swap_total, unixmem_info.swap_used, unixmem_info.swap_free
            );
        } else if args[1] == valid_options[3] {
            header = "              total        used       free      avail  buff/cache";
            memory_content = format!(
                "Mem:        {:.1}GiB     {:.1}GiB     {:.1}GiB     {:.1}GiB      {:.1}GiB",
                unixmem_info.total as f32 / 1024.0,
                unixmem_info.used as f32 / 1024.0,
                unixmem_info.free as f32 / 1024.0,
                unixmem_info.avail as f32 / 1024.0,
                unixmem_info.buff_cache as f32 / 1024.0
            );
            swaped_content = format!(
                "Swap:       {:.1}GiB     {:.1}GiB     {:.1}GiB",
                unixmem_info.swap_total as f32 / 1024.0,
                unixmem_info.swap_used as f32 / 1024.0,
                unixmem_info.swap_free as f32 / 1024.0
            );
        }
    }
    println!("{header}");
    println!("{memory_content}");
    println!("{swaped_content}");
}

fn transform_meminfo_by_option(unixmem_info: &mut UnixMemInfo, option: &String) {
    let mut diviser = 1_u64;
    if option == "-m" || option == "-h" {
        diviser = diviser << 10;
    } else if option == "-g" {
        diviser = diviser << 20;
    }
    unixmem_info.total /= diviser;
    unixmem_info.used /= diviser;
    unixmem_info.free /= diviser;
    unixmem_info.avail /= diviser;
    unixmem_info.buff_cache /= diviser;
    unixmem_info.swap_total /= diviser;
    unixmem_info.swap_used /= diviser;
    unixmem_info.swap_free /= diviser;
}

fn main() {
    let args: Vec<String> = make_args();
    free(args);
}
