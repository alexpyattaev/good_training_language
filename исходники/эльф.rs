use super::Результат;
use std::fs;
use std::path::Path;
use std::io::Write;
use компилятор::{Инструкция, ПП};

#[derive(Debug)]
struct ЗаплаткаЦелейПрыжков {
    адрес_инструкции_прыжка: usize,
    адрес_операнда_прыжка: usize,
    индекс_инструкции_пп_цели: usize,
}

pub fn сгенерировать(путь_к_файлу: &Path, пп: &ПП) -> Результат<()> {
    let размер_заголовков: u64 = 64 + 56;
    let точка_входа: u64 = 0x400000;
    let начало_данных = точка_входа + размер_заголовков;
    let mut код = vec![];

    код.extend(&пп.данные);

    let mut адреса_инструкций_пп: Vec<usize> = Vec::new();
    let mut заплатки_целей_прыжков: Vec<ЗаплаткаЦелейПрыжков> = Vec::new();

    for инструкция in &пп.код {
        адреса_инструкций_пп.push(код.len());
        match инструкция {
            Инструкция::Ноп => {}
            // «Короткое» проталкивание (i8) "\x6A\x7F"
            // «Длинное» проталкивание (i32) "\x68\x00\x00\x00\x00"
            Инструкция::ПротолкнутьЦелое(значение) => {
                assert!(*значение <= i32::MAX as usize);
                код.push(0x68); // push
                код.extend((*значение as i32).to_le_bytes());
                // СДЕЛАТЬ: реализовать поддержу «коротких» проталкиваний для целых чисел.
            }
            Инструкция::ПротолкнутьУказатель(указатель) => {
                let значение = указатель + начало_данных as usize;
                assert!(значение <= i32::MAX as usize);
                код.push(0x68); // push
                код.extend((значение as i32).to_le_bytes());
            }
            Инструкция::ОпределитьЛокальный => {
                todo!("СДЕЛАТЬ: Инструкция::ОпределитьЛокальный")
            }
            Инструкция::СброситьЛокальный => {
                todo!("СДЕЛАТЬ: Инструкция::СброситьЛокальный")
            }
            Инструкция::ВызватьПроцедуру(_) => {
                todo!("Инструкция::ВызватьПроцедуру")
            }
            Инструкция::Записать64 => {
                код.push(0x5E);                 // pop rsi
                код.push(0x58);                 // pop rax
                код.extend([0x48, 0x89, 0x06]); // mov [rsi], rax
            }
            Инструкция::Прочитать64 => {
                код.push(0x5E);                 // pop rsi
                код.extend([0x48, 0x8B, 0x06]); // mov rax, [rsi]
                код.push(0x50);                 // push rax
            }
            Инструкция::ЦелМеньше => {
                код.extend([0x5B]);             // pop rbx
                код.extend([0x58]);             // pop rax
                код.extend([0x48, 0x31, 0xC9]); // xor rcx, rcx
                код.extend([0x48, 0x39, 0xD8]); // cmp rax, rbx
                код.extend([0x0F, 0x92, 0xC1]); // setb cl
                код.extend([0x51]);             // push rcx
                // СДЕЛАТЬ: можно ли использовать условное
                // перемещение для реализации инструкции ЦелМеньше?
            }
            Инструкция::ЦелСложение => {
                код.extend([0x5B]);             // pop rbx
                код.extend([0x58]);             // pop rax
                код.extend([0x48, 0x01, 0xD8]); // add rax, rbx
                код.push(0x50);                 // push rax
            }
            Инструкция::ЛогОтрицание => {
                код.extend([0x48, 0x31, 0xDB]); // xor rbx, rbx
                код.extend([0x58]);             // pop rax
                код.extend([0x48, 0x85, 0xC0]); // test rax, rax
                код.extend([0x0F, 0x94, 0xC3]); // setz bl
                код.extend([0x53]);             // push rbx
            }
            &Инструкция::Прыжок(индекс_инструкции_пп_цели) => {
                код.extend([0xE9]); // jmp
                let адрес_операнда_прыжка = код.len();
                код.extend([0x00, 0x00, 0x00, 0x00]); // Заполняем операнд нулями, т.к. реальный относительный адрес будет известен позже.
                // ВНИМАНИЕ! За каким-то хреном, адрес, относительно
                // которого мы прыгаем, находится ПОСЛЕ текущей
                // инструкции. Спасибо Интел!
                let адрес_инструкции_прыжка = код.len();
                заплатки_целей_прыжков.push(ЗаплаткаЦелейПрыжков {
                    адрес_инструкции_прыжка,
                    адрес_операнда_прыжка,
                    индекс_инструкции_пп_цели,
                });
            }
            &Инструкция::УсловныйПрыжок(индекс_инструкции_пп_цели) => {
                код.extend([0x58]);             // pop rax
                код.extend([0x48, 0x85, 0xC0]); // test rax, rax
                код.extend([0x0F, 0x85]);       // jnz
                let адрес_операнда_прыжка = код.len();
                код.extend([0x00, 0x00, 0x00, 0x00]); // Заполняем операнд нулями, т.к. реальный относительный адрес будет известен позже.
                // ВНИМАНИЕ! За каким-то хреном, адрес, относительно
                // которого мы прыгаем, находится ПОСЛЕ текущей
                // инструкции. Спасибо Интел!
                let адрес_инструкции_прыжка = код.len();
                заплатки_целей_прыжков.push(ЗаплаткаЦелейПрыжков {
                    адрес_инструкции_прыжка,
                    адрес_операнда_прыжка,
                    индекс_инструкции_пп_цели,
                });
            }
            Инструкция::ПечатьСтроки => {
                // SYS_write
                код.extend([0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]); // mov rax, 1
                код.extend([0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00]); // mov rdi, 1
                код.extend([0x5e]);                                     // pop rsi
                код.extend([0x5A]);                                     // pop rdx
                код.extend([0x0F, 0x05]);                               // syscall
            },
            Инструкция::ПечатьЦелого => {
                todo!("СДЕЛАТЬ: генерация машинного кода для инструкции ПечатьЦелого");
            },
            Инструкция::ПечатьЛогического => {
                todo!("СДЕЛАТЬ: генерация машинного кода для инструкции ПечатьЛогического");
            },
            Инструкция::Возврат => {
                // SYS_exit
                код.extend([0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]); // mov rax, 60
                код.extend([0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]); // mov rdi, 0
                код.extend([0x0F, 0x05]); // syscall
            }
        }
    }

    for ЗаплаткаЦелейПрыжков {
        адрес_инструкции_прыжка,
        адрес_операнда_прыжка,
        индекс_инструкции_пп_цели
    } in заплатки_целей_прыжков {
        let операнд = &mut код[адрес_операнда_прыжка..адрес_операнда_прыжка+4];
        let адрес_инструкции_прыжка = адрес_инструкции_прыжка as i32;
        let адрес_инструкции_пп = адреса_инструкций_пп[индекс_инструкции_пп_цели] as i32;
        let относительный_адрес = адрес_инструкции_пп - адрес_инструкции_прыжка;
        операнд.copy_from_slice(&относительный_адрес.to_le_bytes());
    }

    let mut байты: Vec<u8> = Vec::new();
    байты.extend([0x7f, 0x45, 0x4c, 0x46,
                  0x02, 0x01, 0x01, 0x00,
                  0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00]); // e_ident
    байты.extend(2u16.to_le_bytes()); // e_type
    байты.extend(62u16.to_le_bytes()); // e_machine
    байты.extend(1u32.to_le_bytes()); // e_version
    байты.extend((точка_входа + размер_заголовков + пп.данные.len() as u64).to_le_bytes()); // e_entry
    байты.extend(64u64.to_le_bytes()); // e_phoff
    байты.extend(0u64.to_le_bytes()); // e_shoff
    байты.extend(0u32.to_le_bytes()); // e_flags
    байты.extend(64u16.to_le_bytes()); // e_ehsize
    байты.extend(56u16.to_le_bytes()); // e_phentsize
    байты.extend(1u16.to_le_bytes()); // e_phnum
    байты.extend(64u16.to_le_bytes()); // e_shentsize
    байты.extend(0u16.to_le_bytes()); // e_shnum
    байты.extend(0u16.to_le_bytes()); // e_shstrndx

    байты.extend(1u32.to_le_bytes()); // p_type
    байты.extend(7u32.to_le_bytes()); // p_flags
    байты.extend(0u64.to_le_bytes()); // p_offset
    байты.extend(точка_входа.to_le_bytes()); // p_vaddr
    байты.extend(точка_входа.to_le_bytes()); // p_paddr
    байты.extend((размер_заголовков + код.len() as u64).to_le_bytes()); // p_filesz
    байты.extend((размер_заголовков + код.len() as u64).to_le_bytes()); // p_memsz
    байты.extend(4096u64.to_le_bytes()); // p_align

    байты.extend(&код);

    let mut файл = fs::File::create(путь_к_файлу).map_err(|ошибка| {
        eprintln!("ОШИБКА: не удалось открыть файл «{путь_к_файлу}»: {ошибка}",
                  путь_к_файлу = путь_к_файлу.display());
    })?;

    #[cfg(all(unix))] {
        use std::os::unix::fs::PermissionsExt;
        let mut права = файл.metadata().map_err(|ошибка| {
            eprintln!("ОШИБКА: не получилось прочитать метаданные файла «{путь_к_файлу}»: {ошибка}",
                      путь_к_файлу = путь_к_файлу.display());
        })?.permissions();
        права.set_mode(0o755);
        файл.set_permissions(права).map_err(|ошибка| {
            eprintln!("ОШИБКА: не получилось установить права для файла «{путь_к_файлу}»: {ошибка}",
                      путь_к_файлу = путь_к_файлу.display());
        })?;
    }

    match файл.write(&байты) {
        Ok(_) => {
            println!("ИНФО: сгенерирован файл «{путь_к_файлу}»",
                     путь_к_файлу = путь_к_файлу.display());
            Ok(())
        }
        Err(ошибка) => {
            eprintln!("ОШИБКА: не удалось записать файл «{путь_к_файлу}»: {ошибка}",
                      путь_к_файлу = путь_к_файлу.display());
            Err(())
        }
    }
}
