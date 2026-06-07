use crate::control_unit::ControlSignals;

mod control_unit;
mod data_path;
mod general;

// fn main() {
//     let mut memory = [0; 0x10000];
//     memory[0] = 50;
//     memory[1] = 17;

//     let mut cu = control_unit::ControlUnit::new();
//     let mut dp = data_path::DataPath::new(memory);

//     let cs = ControlSignals {
//         mem_read: true,
//         mdr_write_enable: true,
//         ip_write_enable: true,
//         ip_src: control_unit::IpSrc::Increment,
//         ..Default::default()
//     };
//     dp.tick(cs);

//     loop {
//         let cu_signals = cu.tick();
//         dp.tick(cu_signals);
//     }
// }

fn main() {
    let mut memory = [0; 0x10000];
    memory[0] = 50; // Данные для AL
    memory[1] = 17; // Данные для AH

    let mut cu = control_unit::ControlUnit::new();
    let mut dp = data_path::DataPath::new(memory);

    // ЭТОТ ТЕСТ СГЕНЕРИРОВАН. ПИСАТЬ САМОМУ БУРДУ ИЗ ФЛАГОВ ПРОСТО ТАК ЛЕНЬ,
    // А ПРОВЕРИТЬ DATA PATH, ЧТО ОН ХОТЬ КАК-ТО РАБОТАЕТ, ХОЧЕТСЯ
    // РАБОТАЕТ!!!

    // =========================================================================
    // ИНСТРУКЦИЯ 1: MOV AL, 50
    // =========================================================================

    // Такт 1: Читаем memory[MAR=0] в MDR. Инкрементируем IP (IP станет равен 1)
    dp.tick(ControlSignals {
        mem_read: true,
        mdr_write_enable: true,
        ip_write_enable: true,
        ip_src: control_unit::IpSrc::Increment,
        ..Default::default()
    });

    // Такт 2: Переносим 50 из MDR в регистр AL (пробрасываем через ALU)
    // Заодно обновляем MAR значением IP (MAR станет равен 1) для следующего шага
    dp.tick(ControlSignals {
        alu_src_a: control_unit::AluSrcA::Mdr,
        alu_operation: data_path::AluOperation::PassLeft, // Просто пропускаем правый операнд
        rf_write_dst: data_path::GeneralPurposeRegister::AL,
        rf_write_enable: true,
        mar_write_enable: true,
        mar_src: control_unit::MarSrc::Ip,
        ..Default::default()
    });

    // =========================================================================
    // ИНСТРУКЦИЯ 2: MOV AH, 17
    // =========================================================================

    // Такт 3: Читаем memory[MAR=1] в MDR. Инкрементируем IP (IP станет равен 2)
    dp.tick(ControlSignals {
        mem_read: true,
        mdr_write_enable: true,
        ip_write_enable: true,
        ip_src: control_unit::IpSrc::Increment,
        ..Default::default()
    });

    // Такт 4: Переносим 17 из MDR в регистр AH
    dp.tick(ControlSignals {
        alu_src_a: control_unit::AluSrcA::Mdr,
        alu_operation: data_path::AluOperation::PassLeft,
        rf_write_dst: data_path::GeneralPurposeRegister::AH,
        rf_write_enable: true,
        ..Default::default()
    });

    // =========================================================================
    // ИНСТРУКЦИЯ 3: ADD AL, AH
    // =========================================================================

    // Такт 5: Складываем AL и AH, результат сохраняем в AL. Изменяем флаги процессора.
    // Заодно обновляем MAR значением IP (MAR станет равен 2) — это адрес для записи!
    dp.tick(ControlSignals {
        rf_read_a: data_path::GeneralPurposeRegister::AL,
        rf_read_b: data_path::GeneralPurposeRegister::AH,
        alu_src_a: control_unit::AluSrcA::RegisterA,
        alu_src_b: control_unit::AluSrcB::RegisterB,
        alu_operation: data_path::AluOperation::Add,
        rf_write_dst: data_path::GeneralPurposeRegister::AL,
        rf_write_enable: true,
        alu_flags_write_enable: true, // ADD влияет на флаги (ZF, CF, OF...)
        mar_write_enable: true,
        mar_src: control_unit::MarSrc::Ip,
        ..Default::default()
    });

    // =========================================================================
    // ИНСТРУКЦИЯ 4: MOV mem[2], AL
    // =========================================================================

    // Такт 6: Поскольку запись в память идет из MDR, нам нужно сначала
    // положить значение из регистра AL в MDR. Пропустим AL через ALU в MDR.
    dp.tick(ControlSignals {
        rf_read_a: data_path::GeneralPurposeRegister::AL,
        alu_src_a: control_unit::AluSrcA::RegisterA,
        alu_operation: data_path::AluOperation::PassLeft,
        mdr_write_enable: true,
        // Здесь предполагается, что на входе в MDR стоит MUX, который умеет
        // брать данные с выхода ALU при записи в память. Если у тебя для этого
        // есть отдельный сигнал (например mdr_src), укажи его.
        ..Default::default()
    });

    // Такт 7: Запись! Переносим данные из MDR в ячейку памяти по адресу MAR (а MAR равен 2)
    dp.tick(ControlSignals {
        mem_write: true,
        ..Default::default()
    });

    // Проверка результата: 50 + 17 = 67
    // В конце цепочки тактов в памяти по индексу 2 должно лежать число 67.
    // assert_eq!(dp.get_memory()[2], 67);
    println!("{}", dp.get_memory()[2])
}
