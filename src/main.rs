extern crate gtk;
extern crate gio;


use gtk::prelude::*;

use gtk::{ButtonsType, DialogFlags, MessageType, MessageDialog, Window};


use std::sync::mpsc;
use std::{thread, time};
use std::process;

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

#[derive(Debug)]
#[derive(Clone)]
enum OpCode {
    IncrementPointer,
    DecrementPointer,
    Increment,
    Decrement,
    Write,
    Read,
    LoopBegin,
    LoopEnd,
}

#[derive(Debug)]
#[derive(Clone)]
enum Instruction {
    IncrementPointer,
    DecrementPointer,
    Increment,
    Decrement,
    Write,
    Read,
    Loop(Vec<InstructionIndex>),
    Error(String)
}

#[derive(Debug)]
#[derive(Clone)]
enum Action {
    Input,
    Output,
    Tape,
    Error(String)
}

#[derive(Debug)]
#[derive(Clone)]
struct CellChange{
    index: usize,
    content: u8,
    action: Action,
    text_index: usize
}

#[derive(Debug)]
#[derive(Clone)]
struct InstructionIndex{
    index: usize,
    code: Instruction,
}

static RESET: AtomicBool = AtomicBool::new(false);
static PAUSE: AtomicBool = AtomicBool::new(false);
static DELAY: AtomicU16 = AtomicU16::new(500);

//turns the source code into opcodes
fn lex(source: String) -> Vec<(OpCode, usize)> {
    let mut operations = Vec::new();
    let mut cnt = 0;

    for symbol in source.chars() {
        let op = match symbol {
            '>' => Some((OpCode::IncrementPointer, cnt)),
            '<' => Some((OpCode::DecrementPointer, cnt)),
            '+' => Some((OpCode::Increment, cnt)),
            '-' => Some((OpCode::Decrement, cnt)),
            '.' => Some((OpCode::Write, cnt)),
            ',' => Some((OpCode::Read, cnt)),
            '[' => Some((OpCode::LoopBegin, cnt)),
            ']' => Some((OpCode::LoopEnd, cnt)),
            _ => None
        };

        match op {
            Some(op) => operations.push(op),
            None => ()
        }
        cnt+=1;
    }

    operations
}

//turns the opcodes into instructions
fn parse(opcodes: Vec<(OpCode, usize)>) -> Vec<InstructionIndex> {
    let mut program: Vec<InstructionIndex> = Vec::new();
    let mut loop_stack = 0;
    let mut loop_start = 0;

    for (i, op) in opcodes.iter().enumerate() {
        if loop_stack == 0 {
            let instr = match op {

                (OpCode::IncrementPointer, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::IncrementPointer})                 
                },

                (OpCode::DecrementPointer, txt_index)=> {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::DecrementPointer})
                },
                
                (OpCode::Increment, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::Increment})
                },

                (OpCode::Decrement, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::Decrement})
                },

                (OpCode::Write, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::Write})
                },

                (OpCode::Read, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::Read})
                },

                (OpCode::LoopBegin, _) => {
                    loop_start = i;
                    loop_stack += 1;
                    None
                },

                (OpCode::LoopEnd, txt_index) => {
                    Some(InstructionIndex{index: *txt_index, code: Instruction::Error(format!("Loop without beginning at index {}", txt_index))})
                },
            };

            match instr{
                Some(instr) => program.push(instr),
                None => ()
            }
        } 
        else {
            match op {
                (OpCode::LoopBegin,_) => {
                    loop_stack += 1;
                },
                (OpCode::LoopEnd, _) => {
                    loop_stack -= 1;

                    if loop_stack == 0 {
                        program.push(InstructionIndex{index: i, code: Instruction::Loop(parse(opcodes[loop_start+1..i].to_vec()))});
                    }
                },
                _ => (),
            }
        }
    }

    if loop_stack != 0 {
        program.push(InstructionIndex{index: loop_start, code: Instruction::Error(format!("Loop without ending at index {}", loop_start))});
        //panic!("loop that starts at #{} has no matching ending!", loop_start);
    }

    program
}

//runs the parsed program
fn run(instructions: &Vec<InstructionIndex>, tape: &mut Vec<u8>, data_pointer: &mut usize, send_cell: std::sync::mpsc::Sender<CellChange>, receive_data: &std::sync::mpsc::Receiver<i16>) {
    for instr in instructions {
        if RESET.load(Ordering::Relaxed) {
            break;
        }

        if PAUSE.load(Ordering::Relaxed) {
            loop {
                if PAUSE.load(Ordering::Relaxed) == false{
                    break;
                }
                if RESET.load(Ordering::Relaxed) {
                    return;
                }
            }
        }
        thread::sleep(time::Duration::from_millis(DELAY.load(Ordering::Relaxed) as u64));
        match instr {
            
            InstructionIndex{index: i, code: Instruction::IncrementPointer} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }

                if *data_pointer < 31 {
                    *data_pointer += 1;
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i}).unwrap();
                } 
                else {
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(String::from(format!("Data pointer out of bounds at index {}", *i))), text_index: *i}).unwrap();
                    RESET.store(true, Ordering::Relaxed);
                    break;
                }

                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::DecrementPointer} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }

                if *data_pointer >0 {
                    *data_pointer -= 1;
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i}).unwrap();
                }
                else {
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(String::from(format!("Data pointer out of bounds at index {}", *i))), text_index: *i}).unwrap();
                    RESET.store(true, Ordering::Relaxed);
                    break;
                }

                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::Increment} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }

                if tape[*data_pointer] < 255 {
                    tape[*data_pointer] += 1;
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i}).unwrap();
                }
                else {
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(String::from(format!("Addition overflow at index {}", *i))), text_index: *i}).unwrap();
                    RESET.store(true, Ordering::Relaxed);
                    break;
                }
                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::Decrement} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }
                if tape[*data_pointer] > 0 {
                    tape[*data_pointer] -= 1;
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i}).unwrap();
                }
                else {
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(String::from(format!("Subtraction underflow at index {}", *i))), text_index: *i}).unwrap();
                    RESET.store(true, Ordering::Relaxed);
                    break;
                }
                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::Write} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Output, text_index: *i}).unwrap();
                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::Read} => {    
                if RESET.load(Ordering::Relaxed) {
                    break;
                }                  
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Input, text_index: *i}).unwrap();
                if PAUSE.load(Ordering::Relaxed) {
                    loop {
                        if PAUSE.load(Ordering::Relaxed) == false{
                            break;
                        }
                        if RESET.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
                let input = receive_data.recv().unwrap();
                if input != -1 {
                    tape[*data_pointer] = input as u8;
                } 
                else {
                    send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(String::from(format!("invalid input at index {}", *i))), text_index: *i}).unwrap();
                    RESET.store(true, Ordering::Relaxed);
                    break;
                }
            },

            InstructionIndex{index: _, code: Instruction::Loop(nested_instructions)} => {
                if RESET.load(Ordering::Relaxed) {
                    break;
                }
                while tape[*data_pointer] != 0 {
                    run(&nested_instructions, tape, data_pointer, send_cell.clone(), &receive_data);
                    if PAUSE.load(Ordering::Relaxed) {
                        loop {
                            if PAUSE.load(Ordering::Relaxed) == false{
                                break;
                            }
                            if RESET.load(Ordering::Relaxed) {
                                return;
                            }
                        }
                    }
                    if RESET.load(Ordering::Relaxed) {
                        break;
                    }
                }
            },

            InstructionIndex{index: i, code: Instruction::Error(e)} => {    
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Error(e.clone()), text_index: *i}).unwrap();
                RESET.store(true, Ordering::Relaxed);
                break;
            }
        }
    }
}

//does the parsing and visualizing
fn start_parsing(tape_lbls: &Vec<gtk::Label>, marker_lbls: &Vec<gtk::Label>, input: &gtk::TextView, output: &gtk::TextView, speed_slider: &gtk::Scale){

    RESET.store(false, Ordering::Relaxed);
    
    for i in 0..32 {
        tape_lbls[i].set_text("0");
        marker_lbls[i].set_text("");
    }

    input.set_editable(false);
    input.set_cursor_visible(false);

    let buf: gtk::TextBuffer = output.get_buffer().unwrap();
    let in_buf: gtk::TextBuffer = input.get_buffer().unwrap();
    buf.set_text("");
    output.set_buffer(Some(&buf));
    let source_buffer = input.get_buffer().unwrap();
    let source = source_buffer.get_text(&source_buffer.get_start_iter(), &source_buffer.get_end_iter(), false);

    let opcodes = lex(String::from(source.as_ref().unwrap().as_str()));
    let program = parse(opcodes);

    let mut tape: Vec<u8> = vec![0; 32];
    let mut data_pointer = 0;

    let (send_cell, receive_cell) = mpsc::channel();
    let (send_data, receive_data) = mpsc::channel();


    thread::spawn(move || {

        run(&program, &mut tape, &mut data_pointer, send_cell, &receive_data);

        return;
    });
    loop { 
        DELAY.store(speed_slider.get_value()as u16* 10, Ordering::Relaxed);

        let received = receive_cell.try_recv();

        match received.clone(){
            Ok(CellChange{index: _, content: _, action: Action::Output, text_index: i}) => {

                let mut output_txt = String::from(buf.get_text(&buf.get_start_iter(), &buf.get_end_iter(), false).unwrap().as_str());
                in_buf.select_range(&in_buf.get_iter_at_offset(i as i32), &in_buf.get_iter_at_offset(i as i32 +1));

                output_txt.push(received.unwrap().content as char);
                buf.set_text(output_txt.as_str());

                while gtk::events_pending(){
                    gtk::main_iteration();
                }
            },

            Ok(CellChange{index: _, content: _, action: Action::Tape, text_index: i}) => {

                for i in 0..32 {
                    marker_lbls[i].set_text("");
                }

                marker_lbls[received.clone().unwrap().index].set_text("#");
                let tmp = received.clone().unwrap().content.to_string();
                in_buf.select_range(&in_buf.get_iter_at_offset(i as i32), &in_buf.get_iter_at_offset(i as i32 +1));
                tape_lbls[received.clone().unwrap().index].set_text(&tmp[..]);
                while gtk::events_pending(){
                    gtk::main_iteration();
                }
            },

            Ok(CellChange{index: _, content: _, action: Action::Input, text_index: i}) => {

                for i in 0..32 {
                    marker_lbls[i].set_text("");
                }

                marker_lbls[received.clone().unwrap().index].set_text("#");
                let tmp = received.clone().unwrap().content.to_string();
                in_buf.select_range(&in_buf.get_iter_at_offset(i as i32), &in_buf.get_iter_at_offset(i as i32 +1));
                tape_lbls[received.clone().unwrap().index].set_text(&tmp[..]);

                let dialog_window = MessageDialog::new(None::<&Window>, DialogFlags::empty(), MessageType::Info, ButtonsType::Ok, "Input:");

                let dialog_box = dialog_window.get_content_area();
                let user_entry = gtk::Entry::new();

                user_entry.set_size_request(250,0);
                dialog_box.pack_end(&user_entry, false, false, 0);

                dialog_window.show_all();

                let response = dialog_window.run();
                let text = user_entry.get_text();

                let number = text.parse::<i64>();

                match number{
                    Ok(_) => {
                        if number.clone().unwrap() > 255 || number.clone().unwrap() < 0{
                            send_data.send(-1).unwrap();
                        }
                        else {
                            send_data.send(number.unwrap() as i16).unwrap();
                        }
                    },
                    Err(_) => {
                        if (response == gtk::ResponseType::Ok)  && (text.len() == 1){
                            send_data.send(text.as_bytes()[0] as i16).unwrap();
                        }
                        else {
                            send_data.send(-1).unwrap();
                        }
                    }
                }

                dialog_window.close();
                
                while gtk::events_pending(){
                    gtk::main_iteration();
                }
            },

            Ok(CellChange{index: _, content: _, action: Action::Error(e), text_index: _}) => {

                let output_txt = format!("Error: {}", e);

                buf.set_text(output_txt.as_str());

                while gtk::events_pending(){
                    gtk::main_iteration();
                }

            },

            Err(mpsc::TryRecvError::Empty) => {

                while gtk::events_pending(){
                    gtk::main_iteration();
                }

            },

            Err(_) => {
                break;
            }
        }       
    }
    input.set_editable(true);
    input.set_cursor_visible(true);
}

fn reset_app(tape_lbls: &Vec<gtk::Label>, marker_lbls: &Vec<gtk::Label>, input: &gtk::TextView, output: &gtk::TextView){

    RESET.store(true, Ordering::Relaxed);

    for i in 0..32 {
        tape_lbls[i].set_text("0");
        marker_lbls[i].set_text("");
    }

    let buf: gtk::TextBuffer = output.get_buffer().unwrap();
    let in_buf: gtk::TextBuffer = input.get_buffer().unwrap();
    
    buf.set_text("");
    in_buf.set_text("");

    while gtk::events_pending(){
        gtk::main_iteration();
    }
}

//----------------------------------------------------------------TESTS START HERE-------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_parse() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Hello World!\n")
    }

    #[test]
    fn loop_no_start() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("++++++++>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Loop without beginning at index 47")
    }

    #[test]
    fn loop_no_end() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<->>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Loop without ending at index 8")
    }

    #[test]
    fn subtraction_underflow() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("-");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Subtraction underflow at index 0")
    }

    #[test]
    fn addition_overflow() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Addition overflow at index 255")
    }

    #[test]
    fn pointer_out_of_bounds_lower() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("<");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Data pointer out of bounds at index 0")
    }

    #[test]
    fn pointer_out_of_bounds_upper() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Error: Data pointer out of bounds at index 31")
    }

    #[test]
    fn reset() {
        if gtk::init().is_err() {
            println!("Failed to initialize GTK.");
            return;
        }
        let glade_src = include_str!("../GUI.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
        let input: gtk::TextView = builder.get_object("txtInput").unwrap();
        let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
        let tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
        let marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

        let tape_lbls_copy = tape_lbls.clone();
        let marker_lbls_copy = marker_lbls.clone();
        let input_copy = input.clone();
        let output_copy = output.clone();    

        let source_buffer = input.get_buffer().unwrap();
        source_buffer.set_text("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.");

        speed_slider.set_value(0.0);
        
        start_parsing(&tape_lbls, &marker_lbls, &input, &output, &speed_slider);
        
        let out_buffer = output.get_buffer().unwrap();
        let in_buffer = input.get_buffer().unwrap();

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "Hello World!\n");

        reset_app(&tape_lbls_copy, &marker_lbls_copy, &input_copy, &output_copy);

        assert_eq!(String::from(out_buffer.get_text(&out_buffer.get_start_iter(), &out_buffer.get_end_iter(), false).unwrap()), "");
        
        assert_eq!(String::from(in_buffer.get_text(&in_buffer.get_start_iter(), &in_buffer.get_end_iter(), false).unwrap()), "");
    }

}

//----------------------------------------------------------------TESTS END HERE-------------------------------------------------------------


fn main() {

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("../GUI.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::Window = builder.get_object("window").unwrap();
    let start_button: gtk::Button= builder.get_object("btnStart").unwrap();
    let pause_button: gtk::Button = builder.get_object("btnPause").unwrap();
    let step_button: gtk::Button = builder.get_object("btnStep").unwrap();
    let reset_button: gtk::Button = builder.get_object("btnReset").unwrap();
    let speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
    let input: gtk::TextView = builder.get_object("txtInput").unwrap();
    let output: gtk::TextView = builder.get_object("txtOutput").unwrap();
    let mut tape_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
    let mut marker_lbls: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];

    for i in 0..32 {
        tape_lbls[i] = builder.get_object(("lbl".to_owned() + i.to_string().as_str()).as_str()).unwrap();
        marker_lbls[i] = builder.get_object(("lblMarker".to_owned() + i.to_string().as_str()).as_str()).unwrap();
    }

    window.set_title("Brainfuck Visualizer");

    window.show_all();

    let tape_lbls_copy = tape_lbls.clone();
    let marker_lbls_copy = marker_lbls.clone();
    let input_copy = input.clone();
    let output_copy = output.clone();

    let tape_lbls_copy_2 = tape_lbls.clone();
    let marker_lbls_copy_2 = marker_lbls.clone();
    let input_copy_2 = input.clone();
    let output_copy_2 = output.clone();

    start_button.connect_clicked(move |but| {

        but.set_sensitive(false);
        start_parsing(&tape_lbls_copy, &marker_lbls_copy, &input_copy, &output_copy, &speed_slider);
        but.set_sensitive(true);

    });

    reset_button.connect_clicked(move |_|{

        input_copy_2.set_editable(true);
        input_copy_2.set_cursor_visible(true);
        
        reset_app(&tape_lbls_copy_2, &marker_lbls_copy_2, &input_copy_2, &output_copy_2);

    });   

    pause_button.connect_clicked(move |_|{

        if PAUSE.load(Ordering::Relaxed) == false{
            PAUSE.store(true, Ordering::Relaxed);
        }
        else{
            PAUSE.store(false, Ordering::Relaxed);
        }

    });

    step_button.connect_clicked(move |_|{

        PAUSE.store(false, Ordering::Relaxed);
        let delay = DELAY.load(Ordering::Relaxed);

        DELAY.store(10, Ordering::Relaxed);
        thread::sleep(time::Duration::from_millis(5));
        DELAY.store(delay, Ordering::Relaxed);
        PAUSE.store(true, Ordering::Relaxed);

    });

    window.connect_destroy(|_| {
        process::exit(0);
    });

    gtk::main();

}