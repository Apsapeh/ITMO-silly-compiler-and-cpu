mod control_unit;
mod data_path;
mod general;

const IO_READ_PORT: u16 = 0x80;
const IO_WRITE_PORT: u16 = 0x81;

const IO_INPUT_TICK: u64 = 100;
const IO_INPUT_VALUE: u16 = 8;

fn encode_insn(opcode: isa::Opcode, mode: isa::Mode, rd: isa::Register, rs: isa::Register) -> u16 {
    ((opcode as u16) << 10) | ((mode as u16) << 6) | ((rd as u16) << 3) | (rs as u16)
}

fn load_io_irq_test(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;
    use isa::Register::*;

    let loop_addr = 0x0003u16;
    memory[0x0000] = encode_insn(Sti, RegToReg, AL, AL);
    memory[0x0001] = encode_insn(Mov, ImmToReg, SP, AL);
    memory[0x0002] = 0x2000;
    memory[0x0003] = encode_insn(Mov, ImmToMema, AL, AL);
    memory[0x0004] = IO_WRITE_PORT;
    memory[0x0005] = 123;
    memory[0x0006] = encode_insn(Jmp, ImmToReg, AL, AL);
    memory[0x0007] = loop_addr;
    memory[0x0008] = encode_insn(Hlt, RegToReg, AL, AL);

    memory[data_path::IO_IRQ_ADDR as usize] = 0x0100;

    memory[0x0100] = encode_insn(Mov, MemaToReg, AL, AL);
    memory[0x0101] = IO_READ_PORT;
    memory[0x0102] = encode_insn(Add, RegToReg, AL, AL);
    memory[0x0103] = encode_insn(Mov, RegToMema, AL, AL);
    memory[0x0104] = IO_WRITE_PORT;
    memory[0x0105] = encode_insn(Iret, RegToReg, AL, AL);
    // memory[0x0105] = encode_insn(isa::Opcode::Iret, isa::Mode::RegToReg, 0, 0);
}

fn load_io_irq_subcall_test(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;
    use isa::Register::*;

    let loop_addr = 0x0003u16;
    memory[0x0000] = encode_insn(Sti, RegToReg, AL, AL);
    memory[0x0001] = encode_insn(Mov, ImmToReg, SP, AL);
    memory[0x0002] = 0x2000;
    memory[0x0003] = encode_insn(Mov, ImmToReg, AL, AL);
    memory[0x0004] = 123;
    memory[0x0005] = encode_insn(Push, RegToReg, AL, AL);
    memory[0x0006] = encode_insn(Call, ImmToReg, AL, AL);
    memory[0x0007] = 0x0200;
    memory[0x0008] = encode_insn(Jmp, ImmToReg, AL, AL);
    memory[0x0009] = loop_addr;
    memory[0x000A] = encode_insn(Hlt, RegToReg, AL, AL);

    memory[data_path::IO_IRQ_ADDR as usize] = 0x0100;

    memory[0x0100] = encode_insn(Enter, ImmToReg, AL, AL);
    memory[0x0101] = 0;
    memory[0x0102] = encode_insn(Push, RegToReg, AL, AL);
    memory[0x0103] = encode_insn(Mov, MemaToReg, AL, AL);
    memory[0x0104] = IO_READ_PORT;
    memory[0x0105] = encode_insn(Add, RegToReg, AL, AL);
    memory[0x0106] = encode_insn(Push, RegToReg, AL, AL);
    memory[0x0107] = encode_insn(Call, ImmToReg, AL, AL);
    memory[0x0108] = 0x0200;
    memory[0x0109] = encode_insn(Pop, RegToReg, AL, AL);
    memory[0x010A] = encode_insn(Leave, RegToReg, AL, AL);
    memory[0x010B] = encode_insn(Iret, RegToReg, AL, AL);

    // double_and_print (to_print: word)
    memory[0x0200] = encode_insn(Enter, ImmToReg, AL, AL);
    memory[0x0201] = 0;
    memory[0x0202] = encode_insn(Push, RegToReg, AL, AL);
    memory[0x0203] = encode_insn(Mov, MemraToReg, AL, BP);
    memory[0x0204] = 2;
    memory[0x0205] = encode_insn(Add, RegToReg, AL, AL);
    memory[0x0206] = encode_insn(Mov, RegToMema, AL, AL);
    memory[0x0207] = IO_WRITE_PORT;
    memory[0x0208] = encode_insn(Pop, RegToReg, AL, AL);
    memory[0x0209] = encode_insn(Leave, RegToReg, AL, AL);
    memory[0x020A] = encode_insn(Ret, ImmToReg, AL, AL);
    memory[0x020B] = 1;
    // memory[0x0105] = encode_insn(isa::Opcode::Iret, isa::Mode::RegToReg, 0, 0);
}

fn load_add_test(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;
    use isa::Register::*;

    memory[0x0] = encode_insn(Jmp, ImmToReg, AL, AL);
    memory[0x1] = 0x1001;
    memory[0x1000] = 50;
    memory[0x1001] = encode_insn(Add, ImmToMema, AL, AL);
    memory[0x1002] = 0x1000;
    memory[0x1003] = 17;
    memory[0x1004] = encode_insn(Hlt, RegToReg, AL, AL);
}

// This shit is vibe coded, it's just for complex test
// And I spent about 2 hours to debug this AI Slop. Fucking clankers.
// So, bug was at memory[0x020C] (was 0x0216 instead of 0x0214)
// My code works great!
fn load_factorial_interrupt_driven(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;
    use isa::Register::*;

    const IO_READ_PORT: u16 = 0x80;
    const IO_WRITE_PORT: u16 = 0x81;

    // --- 1. ГЛАВНАЯ ПРОГРАММА (MAIN) ---
    memory[0x0000] = encode_insn(Sti, RegToReg, AL, AL); // Разрешаем прерывания
    memory[0x0001] = encode_insn(Mov, ImmToReg, SP, AL); // Инициализация указателя стека
    memory[0x0002] = 0x4000; // Стек в старших адресах
    // Бесконечный цикл ожидания прерывания (аналог loop_addr из твоего примера)
    memory[0x0003] = encode_insn(Jmp, ImmToReg, AL, AL);
    memory[0x0004] = 0x0003;

    // --- 2. УСТАНОВКА ВЕКТОРА ПРЕРЫВАНИЯ ---
    // Записываем адрес обработчика ввода в таблицу векторов (по адресу 0x10)
    memory[data_path::IO_IRQ_ADDR as usize] = 0x0100;

    // --- 3. ОБРАБОТЧИК ПРЕРЫВАНИЯ ВВОДА (IO IRQ) ---
    // Адрес начала: 0x0100
    memory[0x0100] = encode_insn(Enter, ImmToReg, AL, AL); // Создаем фрейм
    memory[0x0101] = 0;

    // Читаем входное значение N из порта в 16-битный регистр AL
    memory[0x0102] = encode_insn(Mov, MemaToReg, AL, AL);
    memory[0x0103] = IO_READ_PORT;

    // memory[0x0104] = encode_insn(Mov, RegToMema, AL, AL);
    // memory[0x0105] = IO_WRITE_PORT;
    // memory[0x0106] = encode_insn(Hlt, RegToReg, AL, AL);

    // Передаем N как аргумент через стек для функции факториала
    memory[0x0104] = encode_insn(Push, RegToReg, AL, AL);

    // Вызываем рекурсивную функцию
    memory[0x0105] = encode_insn(Call, ImmToReg, AL, AL);
    memory[0x0106] = 0x0200; // Адрес функции fact

    // Функция вернет результат в AL и САМА очистит стек (благодаря аппаратной Ret #1).
    // Отправляем результат (факториал) в порт вывода
    memory[0x0107] = encode_insn(Mov, RegToMema, AL, AL);
    memory[0x0108] = IO_WRITE_PORT;

    // Уничтожаем кадр обработчика прерывания
    memory[0x0109] = encode_insn(Leave, RegToReg, AL, AL);

    // Согласно условию: "напечатать вывод и завершиться через hlt"
    memory[0x010A] = encode_insn(Hlt, RegToReg, AL, AL);

    // --- 4. РЕКУРСИВНАЯ ФУНКЦИЯ FACT(N) ---
    // Адрес начала: 0x0200
    // Так как регистры 16-битные, пара AH:AL больше не нужна. Все расчеты делаем в AL.
    // Аргумент N по-прежнему стабильно лежит в кадре по адресу [BP + 2]

    memory[0x0200] = encode_insn(Enter, ImmToReg, AL, AL); // Создаем фрейм стека
    memory[0x0201] = 0;

    // Сохраняем BL, так как будем использовать его для умножения (Callee-save)
    memory[0x0202] = encode_insn(Push, RegToReg, AL, BL);

    // Читаем аргумент N из стека [BP + 2] в регистр AL
    memory[0x0203] = encode_insn(Mov, MemraToReg, AL, BP);
    memory[0x0204] = 2;

    // Базовый случай: Проверяем, равен ли N единице или нулю (N <= 1)
    memory[0x0205] = encode_insn(Cmp, ImmToReg, AL, AL);
    memory[0x0206] = 1;

    // Если N > 1, прыгаем на шаг рекурсии
    memory[0x0207] = encode_insn(Jg, ImmToReg, AL, AL);
    memory[0x0208] = 0x020D; // Адрес шага рекурсии

    // --- Базовый случай (N <= 1) ---
    memory[0x0209] = encode_insn(Mov, ImmToReg, AL, AL);
    memory[0x020A] = 1; // Возвращаем результат 1 в AL
    memory[0x020B] = encode_insn(Jmp, ImmToReg, AL, AL);
    memory[0x020C] = 0x0214; // Прыгаем в эпилог на выход

    // --- Шаг рекурсии (N > 1) ---
    // Адрес: 0x020D. В AL сейчас находится текущий N.
    memory[0x020D] = encode_insn(Dec, RegToReg, AL, AL); // AL = N - 1

    // Передаем (N - 1) как аргумент в стек для следующего вызова
    memory[0x020E] = encode_insn(Push, RegToReg, AL, AL);

    // // Рекурсивный вызов: fact(N - 1)
    memory[0x020F] = encode_insn(Call, ImmToReg, AL, AL);
    memory[0x0210] = 0x0200; // Переход на начало fact

    // Возврат из рекурсии. В AL теперь лежит результат fact(N - 1).
    // Нам нужно достать наш исходный N, который лежит в стеке текущего кадра
    memory[0x0211] = encode_insn(Mov, MemraToReg, BL, BP);
    memory[0x0212] = 2; // Читаем N из [BP + 2] в BL

    // Перемножаем: AL = AL * BL
    memory[0x0213] = encode_insn(Mul, RegToReg, AL, BL);

    // --- ЭПИЛОГ (ВЫХОД ИЗ ФУНКЦИИ) ---
    // Адрес: 0x0214
    memory[0x0214] = encode_insn(Pop, RegToReg, BL, AL); // Восстанавливаем BL
    memory[0x0215] = encode_insn(Leave, RegToReg, AL, AL); // Восстанавливаем SP и BP

    // Твоя новая аппаратная поддержка Ret #n
    memory[0x0216] = encode_insn(Ret, ImmToReg, AL, AL);
    memory[0x0217] = 1; // Извлекает IP и удаляет 1 аргумент
}

fn main() {
    let mut memory = [0u16; 0x10000];
    // load_io_irq_test(&mut memory);
    // load_io_irq_subcall_test(&mut memory);
    // load_add_test(&mut memory);
    load_factorial_interrupt_driven(&mut memory);
    //
    println!("Mem[0x1000..0x1010]: {:?}", &memory[0x0..0x110]);
    println!("Mem[0x1FF0..0x2010]: {:?}", &memory[0x1FF0..0x2010]);

    let mut cu = control_unit::ControlUnit::new();
    let mut dp = data_path::DataPath::new(memory);

    let mut tick = 0u64;
    let max_ticks = 300000;

    while tick < max_ticks && !cu.get_is_halted() {
        if tick == 10 {
            dp.set_io_read_buffer(IO_INPUT_VALUE);
        }
        println!("\nTick {}", tick);

        let signals = cu.tick(&dp);
        // println!("Signal: {:#?}", signals);
        dp.tick(signals);
        tick += 1;
    }

    let new_memory = dp.get_memory();
    println!("Mem[0x1000..0x1004]: {:?}", &new_memory[0x0..0x110]);
    println!("Mem[0x1FF0..0x2010]: {:?}", &new_memory[0x1FF0..0x2010]);

    let output = dp.get_io_write_data();

    println!("Memory: {:?}", dp.get_io_write_data());

    println!(
        "OK: tick {IO_INPUT_TICK} input={IO_INPUT_VALUE} -> output={} (total ticks={tick})",
        output[0]
    );
}
