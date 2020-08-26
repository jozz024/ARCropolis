use std::slice;
use std::fs::File;
use std::sync::atomic::AtomicU32;

use crate::log;
use crate::resource::*;
use crate::replacement_files::ARC_FILES;

use skyline::nn;
use skyline::hooks::{getRegionAddress, Region};

// default 8.0.0 offsets
pub static mut IDK_OFFSET: usize = 0x32545a0;
pub static mut ADD_IDX_TO_TABLE1_AND_TABLE2_OFFSET: usize = 0x324e9f0;
pub static mut LOOKUP_STREAM_HASH_OFFSET: usize = 0x324f7a0;
pub static mut LOADED_TABLES_ADRP_OFFSET: usize = 0x324c3a0;

static IDK_SEARCH_CODE: &[u8] = &[
    0xf8, 0x5f, 0xbc, 0xa9, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b, 0x03, 0xa9,
    0xfd, 0xc3, 0x00, 0x91, 0xe8, 0x5f, 0x00, 0x32, 0x3f, 0x00, 0x08, 0x6b,
];

static ADD_IDX_TO_TABLE1_AND_TABLE2_SEARCH_CODE: &[u8] = &[
    0xf6, 0x57, 0xbd, 0xa9, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91,
    0x08, 0x18, 0x40, 0xb9, 0x1f, 0x01, 0x01, 0x6b,
];

static LOADED_TABLES_ADRP_SEARCH_CODE: &[u8] = &[
    0x28, 0x4b, 0x40, 0xb9, 0xf4, 0x03, 0x01, 0x2a, 0x1f, 0x01, 0x01, 0x6b, 0x29, 0x0a, 0x00, 0x54,
    0x36, 0x03, 0x40, 0xf9, 0xe0, 0x03, 0x16, 0xaa,
];

static LOOKUP_STREAM_HASH_SEARCH_CODE: &[u8] = &[
    0x29, 0x58, 0x40, 0xf9, 0x28, 0x60, 0x40, 0xf9, 0x2a, 0x05, 0x40, 0xb9, 0x09, 0x0d, 0x0a, 0x8b,
    0xaa, 0x01, 0x00, 0x34, 0x5f, 0x01, 0x00, 0xf1,
];

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn offset_from_adrp(adrp_offset: usize) -> usize {
    unsafe {
        let adrp = *(offset_to_addr(adrp_offset) as *const u32);
        let immhi = (adrp & 0b0_00_00000_1111111111111111111_00000) >> 3;
        let immlo = (adrp & 0b0_11_00000_0000000000000000000_00000) >> 29;
        let imm = ((immhi | immlo) << 12) as i32 as usize;
        let base = adrp_offset & 0xFFFFFFFFFFFFF000;
        base + imm
    }
}

fn offset_from_ldr(ldr_offset: usize) -> usize {
    unsafe {
        let ldr = *(offset_to_addr(ldr_offset) as *const u32);
        let size = (ldr & 0b11_000_0_00_00_000000000000_00000_00000) >> 30;
        let imm = (ldr & 0b00_000_0_00_00_111111111111_00000_00000) >> 10;
        (imm as usize) << size
    }
}

pub fn search_offsets() {
    unsafe {
        let text_ptr = getRegionAddress(Region::Text) as *const u8;
        let text_size = (getRegionAddress(Region::Rodata) as usize) - (text_ptr as usize);
        let text = std::slice::from_raw_parts(text_ptr, text_size);
        if let Some(offset) = find_subsequence(text, IDK_SEARCH_CODE) {
            IDK_OFFSET = offset
        } else {
            println!("Error: no offset found for function 'idk'. Defaulting to 8.0.0 offset. This likely won't work.");
        }

        if let Some(offset) = find_subsequence(text, ADD_IDX_TO_TABLE1_AND_TABLE2_SEARCH_CODE) {
            ADD_IDX_TO_TABLE1_AND_TABLE2_OFFSET = offset
        } else {
            println!("Error: no offset found for function 'add_idx_to_table1_and_table2'. Defaulting to 8.0.0 offset. This likely won't work.");
        }

        if let Some(offset) = find_subsequence(text, LOOKUP_STREAM_HASH_SEARCH_CODE) {
            LOOKUP_STREAM_HASH_OFFSET = offset
        } else {
            println!("Error: no offset found for function 'add_idx_to_table1_and_table2'. Defaulting to 8.0.0 offset. This likely won't work.");
        }

        if let Some(offset) = find_subsequence(text, LOADED_TABLES_ADRP_SEARCH_CODE) {
            LOADED_TABLES_ADRP_OFFSET = offset - 8
        } else {
            println!("Error: no offset found for 'loaded_tables_adrp'. Defaulting to 8.0.0 offset. This likely won't work.");
        }
        let adrp_offset = offset_from_adrp(LOADED_TABLES_ADRP_OFFSET);
        let ldr_offset = offset_from_ldr(LOADED_TABLES_ADRP_OFFSET + 4);
        LOADED_TABLES_OFFSET = adrp_offset + ldr_offset;
    }
}

// #[allow(dead_code)]
// pub fn expand_table2() {
//     let loaded_tables = LoadedTables::get_instance();

//     unsafe {
//         nn::os::LockMutex(loaded_tables.mutex);
//     }

//     let mut table2_vec = loaded_tables.table_2().to_vec();

//     table2_vec.push(Table2Entry {
//         data: 0 as *const u8,
//         ref_count: AtomicU32::new(0),
//         is_used: false,
//         state: FileState::Unused,
//         file_flags2: false,
//         flags: 45,
//         version: 0xFFFF,
//         unk: 0,
//     });

//     loaded_tables.table2_len = table2_vec.len() as u32;
//     let mut table2_array = table2_vec.into_boxed_slice();
//     loaded_tables.table2 = table2_array.as_ptr() as *mut Table2Entry;

//     unsafe {
//         nn::os::UnlockMutex(loaded_tables.mutex);
//     }
// }

pub fn filesize_replacement() {
    for (hash, path) in ARC_FILES.iter() {
        let loaded_tables = LoadedTables::get_instance();

        unsafe {
            let hashindexgroup_slice = slice::from_raw_parts(
                loaded_tables.get_arc().file_info_path,
                (*loaded_tables).table1_len as usize,
            );

            let t1_index = hashindexgroup_slice
                .iter()
                .position(|x| x.path.hash40.as_u64() == *hash)
                .unwrap() as u32;

            let mut subfile = loaded_tables.get_arc().get_subfile_by_t1_index(t1_index);

            let file = File::open(path).ok().unwrap();
            let metadata = file.metadata().ok().unwrap();

            subfile.compressed_size = metadata.len() as u32;
            subfile.decompressed_size = metadata.len() as u32;

            log!(
                "[ARC::Patching] New decompressed size for {}: {:#x}",
                path.as_path().display(),
                subfile.decompressed_size
            );
        }
    }
}