extern crate sys_info;
use sys_info::MemInfo;
use regex::Regex;
//use std::io;
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

fn init_unixmeminfo(mem_info: MemInfo)-> UnixMemInfo{
    let swap_used = mem_info.swap_total - mem_info.swap_free;
    let [buff_cache, used] = extract_ex_meminfo();
    UnixMemInfo {
        total: mem_info.total,
        used: used,
        free: mem_info.free,
        avail: mem_info.avail,
        buff_cache: buff_cache,
        swap_total: mem_info.swap_total,
        swap_used: swap_used,
        swap_free: mem_info.swap_free,
    }
}

fn extract_ex_meminfo() -> [u64; 2]{
    // vm_stat
    let output = Command::new("vm_stat")
                         .output()
                         .expect("failed to execute vm_stat");
    // transform into str
    let output_str = String::from_utf8_lossy(&output.stdout);

    let lines = output_str.lines();

    // Define regex to extract u64 numbers.
    let re = Regex::new(r"\d+").unwrap();
    let mut page_size = 0_u64;
    let mut page_cache = 0_u64;
    let mut used = 0_u64;
    let used_memory_columns = ["Pages wired down", "Pages active", "Pages inactive", "Pages speculative", "Pages occupied by compressor"];
    for (idx, line) in lines.enumerate() {
        if idx == 0 {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            page_size = extracted_num;
        } else if line.contains("Pages purgeable") || line.contains("File-backed pages") {
            let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
            page_cache += extracted_num * page_size;
        }
        for column_name in used_memory_columns {
            if is_used_memory(line, column_name) {
                let extracted_num: u64 = re.find(line).unwrap().as_str().parse().unwrap();
                used += extracted_num * page_size;
            }
        }
        
    }
    used -= page_cache; // substract page_cache to calculate truly used memories
    page_cache = page_cache >> 10; // bytes -> kilo bytes
    used = used >> 10;
    [page_cache, used]
}

fn is_used_memory(line: &str, column_name: &str) -> bool {
    if line.contains(column_name) {
        return true;
    }
    false
}

fn free(options: String){
    let mem_info = sys_info::mem_info().unwrap();
    format_mem_info(mem_info, options);
}

/*
fn parse_args()->String{
    //Implement later
}
*/

fn format_mem_info(mem_info: MemInfo, options: String){
    // header, not swaped, swapedの順に出力する
    let header = "               total         used       free       avail   buff/cache";
    let mut unixmem_info: UnixMemInfo = init_unixmeminfo(mem_info); // This variable is mutable because options can change the value.
    let memory_content = format!("Mem:        {}     {}     {}     {}      {}", unixmem_info.total, unixmem_info.used, unixmem_info.free, unixmem_info.avail, unixmem_info.buff_cache);
    let swaped_content = format!("Swap:       {}     {}     {}", unixmem_info.swap_total, unixmem_info.swap_used, unixmem_info.swap_free);
    println!("{header}");
    println!("{memory_content}");
    println!("{swaped_content}");
}

fn main() {
    free("".to_string());
}