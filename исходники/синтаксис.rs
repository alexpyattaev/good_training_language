use std::num::{IntErrorKind};
use лексика::*;
use диагностика::*;
use super::Результат;

#[derive(Clone)]
pub struct Переменная {
    pub имя: Лексема,
    pub тип: Выражение,
}

impl Переменная {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Переменная> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Двоеточие])?;
        let тип = Выражение::разобрать(лекс)?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
        Ok(Переменная{имя, тип})
    }
}

#[derive(Debug, Clone)]
pub enum ВидБинопа {
    Меньше,
    Больше,
    Сложение,
    Вычитание,
    Деление,
    Остаток,
    Равно,
    Как,
}

impl ВидБинопа {
    fn по_виду_лексемы(вид: &ВидЛексемы) -> Option<ВидБинопа> {
        match вид {
            ВидЛексемы::КлючМн => Some(ВидБинопа::Меньше),
            ВидЛексемы::КлючБл => Some(ВидБинопа::Больше),
            ВидЛексемы::Плюс => Some(ВидБинопа::Сложение),
            ВидЛексемы::Минус => Some(ВидБинопа::Вычитание),
            ВидЛексемы::ПрямаяНаклонная => Some(ВидБинопа::Деление),
            ВидЛексемы::Процент => Some(ВидБинопа::Остаток),
            ВидЛексемы::РавноРавно => Some(ВидБинопа::Равно),
            ВидЛексемы::КлючКак => Some(ВидБинопа::Как),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Выражение {
    Число(Лексема, usize),
    Строка(Лексема),
    Идент(Лексема),
    Вызов{имя: Лексема, аргументы: Vec<Выражение>},
    Биноп {
        ключ: Лексема,
        вид: ВидБинопа,
        левое: Box<Выражение>,
        правое: Box<Выражение>,
    }
}

impl Выражение {
    pub fn лок(&self) -> &Лок {
        match self {
            Выражение::Число(лексема, _) |
            Выражение::Строка(лексема)   |
            Выражение::Идент(лексема) => &лексема.лок,
            Выражение::Биноп{ключ, ..} => &ключ.лок,
            Выражение::Вызов{имя, ..} => &имя.лок,
        }
    }

    fn разобрать_первичное(лекс: &mut Лексер) -> Результат<Выражение> {
        let лексема = лекс.вытащить_лексему_вида(&[
            ВидЛексемы::Число,
            ВидЛексемы::Идент,
            ВидЛексемы::Строка,
            ВидЛексемы::ОткрытаяСкобка,
        ])?;
        match лексема.вид {
            ВидЛексемы::Число => {
                match лексема.текст.parse() {
                    Ok(число) => Ok(Выражение::Число(лексема, число)),
                    Err(ошибка) => match ошибка.kind() {
                        IntErrorKind::PosOverflow => {
                            диагностика!(&лексема.лок, "ОШИБКА", "Число слишком большое");
                            Err(())
                        }
                        IntErrorKind::Empty => unreachable!(),
                        IntErrorKind::InvalidDigit => unreachable!(),
                        IntErrorKind::NegOverflow => unreachable!(),
                        IntErrorKind::Zero => unreachable!(),
                        _ => {
                            диагностика!(&лексема.лок, "ОШИБКА", "Число некорректно");
                            Err(())
                        }
                    }
                }
            }
            ВидЛексемы::Идент => {
                let имя = лексема;
                if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ОткрытаяСкобка {
                    let _ = лекс.вытащить_лексему().unwrap();
                    let аргументы = разобрать_список_аргументов_вызова(лекс)?;
                    Ok(Выражение::Вызов{имя, аргументы})
                } else {
                    Ok(Выражение::Идент(имя))
                }
            },
            ВидЛексемы::Строка => Ok(Выражение::Строка(лексема)),
            ВидЛексемы::ОткрытаяСкобка => {
                let выражение = Выражение::разобрать(лекс)?;
                let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ЗакрытаяСкобка])?;
                Ok(выражение)
            }
            _ => unreachable!(),
        }
    }

    fn разобрать_биноп(лекс: &mut Лексер) -> Результат<Выражение> {
        let левое = Выражение::разобрать_первичное(лекс)?;
        if let Some(вид) = ВидБинопа::по_виду_лексемы(&лекс.подсмотреть_лексему()?.вид) {
            let ключ = лекс.вытащить_лексему().unwrap();
            let правое = Выражение::разобрать_биноп(лекс)?;
            Ok(Выражение::Биноп {
                вид,
                ключ,
                левое: Box::new(левое),
                правое: Box::new(правое),
            })
        } else {
            Ok(левое)
        }
    }

    fn разобрать(лекс: &mut Лексер) -> Результат<Выражение> {
        Выражение::разобрать_биноп(лекс)
    }
}

#[derive(Debug)]
pub enum Утверждение {
    Присваивание{ключ: Лексема, имя: Лексема, значение: Выражение},
    ПрисваиваниеМассива{ключ: Лексема, имя: Лексема, индекс: Выражение, значение: Выражение},
    Вызов{имя: Лексема, аргументы: Vec<Выражение>},
    Пока{ключ: Лексема, условие: Выражение, тело: Vec<Утверждение>},
    Если{ключ: Лексема, условие: Выражение, тело: Vec<Утверждение>, иначе: Vec<Утверждение>},
    Вернуть{ключ: Лексема},
}

pub struct Параметр {
    pub имя: Лексема,
    pub тип: Выражение,
}

pub enum ТелоПроцедуры {
    Внутренее { блок: Vec<Утверждение> },
    Внешнее { символ: Лексема },
}

pub struct Процедура {
    pub имя: Лексема,
    pub параметры: Vec<Параметр>,
    pub тело: ТелоПроцедуры,
}

fn разобрать_утверждение(лекс: &mut Лексер) -> Результат<Утверждение> {
    let лексема = лекс.вытащить_лексему_вида(&[
        ВидЛексемы::Идент,
        ВидЛексемы::КлючПока,
        ВидЛексемы::КлючЕсли,
        ВидЛексемы::КлючВернуть,
    ])?;
    match лексема.вид {
        ВидЛексемы::Идент => {
            let имя = лексема;
            let лексема = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::Присваивание,
                ВидЛексемы::ОткрытаяСкобка,
                ВидЛексемы::ОткрытаяКвадСкобка,
            ])?;
            match лексема.вид {
                ВидЛексемы::Присваивание => {
                    let ключ = лексема;
                    let значение = Выражение::разобрать(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                    Ok(Утверждение::Присваивание {ключ, имя, значение})
                }
                ВидЛексемы::ОткрытаяСкобка => {
                    let аргументы = разобрать_список_аргументов_вызова(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                    Ok(Утверждение::Вызов {имя, аргументы})
                }
                ВидЛексемы::ОткрытаяКвадСкобка => {
                    let индекс = Выражение::разобрать(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ЗакрытаяКвадСкобка])?;
                    let ключ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Присваивание])?;
                    let значение = Выражение::разобрать(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                    Ok(Утверждение::ПрисваиваниеМассива {ключ, имя, индекс, значение})
                }
                _ => unreachable!()
            }
        }
        ВидЛексемы::КлючЕсли => {
            let ключ = лексема;
            let условие = Выражение::разобрать(лекс)?;
            let тело = разобрать_блок_кода(лекс)?;
            let иначе;
            if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючИначе {
                let _ = лекс.вытащить_лексему()?;
                иначе = разобрать_блок_кода(лекс)?;
            } else {
                иначе = vec![]
            }
            Ok(Утверждение::Если{ключ, условие, тело, иначе})
        }
        ВидЛексемы::КлючПока => {
            let ключ = лексема;
            let условие = Выражение::разобрать(лекс)?;
            let тело = разобрать_блок_кода(лекс)?;
            Ok(Утверждение::Пока{ключ, условие, тело})
        }
        ВидЛексемы::КлючВернуть => {
            let ключ = лексема;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
            Ok(Утверждение::Вернуть{ключ})
        },
        _ => unreachable!(),
    }
}

fn разобрать_блок_кода(лекс: &mut Лексер) -> Результат<Vec<Утверждение>> {
    let mut блок = Vec::new();
    let ключ = лекс.вытащить_лексему_вида(&[ВидЛексемы::КлючНч, ВидЛексемы::КлючТо])?;
    match ключ.вид {
        ВидЛексемы::КлючНч => loop {
            if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючКц {
                let _ = лекс.вытащить_лексему()?;
                break;
            }
            блок.push(разобрать_утверждение(лекс)?);
        }
        ВидЛексемы::КлючТо => блок.push(разобрать_утверждение(лекс)?),
        _ => unreachable!()
    }
    Ok(блок)
}

fn разобрать_список_аргументов_вызова(лекс: &mut Лексер) -> Результат<Vec<Выражение>> {
    let mut аргументы = Vec::new();

    // СДЕЛАТЬ: ввести идиому лекс.вытащить_лексему_если()
    if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ЗакрытаяСкобка {
        let _ = лекс.вытащить_лексему()?;
    } else {
        'разбор_аргументов: loop {
            аргументы.push(Выражение::разобрать(лекс)?);
            let лексема = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::ЗакрытаяСкобка,
                ВидЛексемы::Запятая
            ])?;
            if лексема.вид == ВидЛексемы::ЗакрытаяСкобка {
                break 'разбор_аргументов
            }
        }
    }
    Ok(аргументы)
}

fn разобрать_список_параметров_процедуры(лекс: &mut Лексер) -> Результат<Vec<Параметр>> {
    let mut параметры: Vec<Параметр> = Vec::new();
    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ОткрытаяСкобка])?;
    if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ЗакрытаяСкобка {
        let _ = лекс.вытащить_лексему()?;
    } else {
        'разбор_параметров: loop {
            let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
            if let Some(существующий_параметр) = параметры.iter().find(|параметр| параметр.имя.текст == имя.текст) {
                диагностика!(&имя.лок, "ОШИБКА", "переопределение параметра «{имя}»",
                             имя = имя.текст);
                диагностика!(&существующий_параметр.имя.лок, "ИНФО", "параметр с тем же именем определен тут");
                return Err(());
            }
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Двоеточие])?;
            let тип = Выражение::разобрать(лекс)?;
            параметры.push(Параметр {имя, тип});
            let лексема = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::ЗакрытаяСкобка,
                ВидЛексемы::Запятая
            ])?;
            if лексема.вид == ВидЛексемы::ЗакрытаяСкобка {
                break 'разбор_параметров
            }
        }
    }
    Ok(параметры)
}

impl Процедура {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Процедура> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let параметры = разобрать_список_параметров_процедуры(лекс)?;
        let тело = if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючВнешняя {
            let _ = лекс.вытащить_лексему().unwrap();
            let символ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Строка])?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
            ТелоПроцедуры::Внешнее {символ}
        } else {
            let блок = разобрать_блок_кода(лекс)?;
            ТелоПроцедуры::Внутренее {блок}
        };
        Ok(Процедура{имя, параметры, тело})
    }
}

#[derive(Debug)]
pub struct Константа {
    pub имя: Лексема,
    pub выражение: Выражение,
}

impl Константа {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Константа> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Равно])?;
        let выражение = Выражение::разобрать(лекс)?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
        Ok(Константа{имя, выражение})
    }
}
