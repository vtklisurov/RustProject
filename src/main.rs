extern crate gtk;
extern crate gio;


use gtk::prelude::*;
use gio::prelude::*;

use gtk::{Application, ApplicationWindow, Button};
use gtk::{ButtonsType, DialogFlags, MessageType, MessageDialog, Window};

use std::env;
use std::io::Read;
use std::sync::mpsc;
use std::{thread, time};

use std::sync::atomic::{AtomicBool, Ordering};

/// Opcodes determined by the lexer
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
    Loop(Vec<InstructionIndex>)
}

#[derive(Debug)]
#[derive(Clone)]
enum Action {
    Input,
    Output,
    Tape
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

/// Lexer turns the source code into a sequence of opcodes
fn lex(source: String) -> Vec<OpCode> {
    let mut operations = Vec::new();

    for symbol in source.chars() {
        let op = match symbol {
            '>' => Some(OpCode::IncrementPointer),
            '<' => Some(OpCode::DecrementPointer),
            '+' => Some(OpCode::Increment),
            '-' => Some(OpCode::Decrement),
            '.' => Some(OpCode::Write),
            ',' => Some(OpCode::Read),
            '[' => Some(OpCode::LoopBegin),
            ']' => Some(OpCode::LoopEnd),
            _ => None
        };

        // Non-opcode characters are simply comments
        match op {
            Some(op) => operations.push(op),
            None => ()
        }
    }

    operations
}

fn parse(opcodes: Vec<OpCode>, txt_index: &mut usize) -> Vec<InstructionIndex> {
    let mut program: Vec<InstructionIndex> = Vec::new();
    let mut loop_stack = 0;
    let mut loop_start = 0;

    for (i, op) in opcodes.iter().enumerate() {
        if loop_stack == 0 {
            let instr = match op {
                OpCode::IncrementPointer => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::IncrementPointer})                 
                },
                OpCode::DecrementPointer => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::DecrementPointer})
                },
                OpCode::Increment => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Increment})
                },
                OpCode::Decrement => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Decrement})
                },
                OpCode::Write => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Write})
                },
                OpCode::Read => {
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Read})
                },

                OpCode::LoopBegin => {
                    loop_start = i;
                    loop_stack += 1;
                    None
                },

                OpCode::LoopEnd => {
                    panic!("loop ending at #{} has no beginning", i)
                },
            };

            match instr{
                Some(instr) => program.push(instr),
                None => ()
            }
        } else {
            match op {
                OpCode::LoopBegin => {
                    loop_stack += 1;
                },
                OpCode::LoopEnd => {
                    loop_stack -= 1;

                    if loop_stack == 0 {
                        *txt_index += 1;
                        program.push(InstructionIndex{index: i, code: Instruction::Loop(parse(opcodes[loop_start+1..i].to_vec(), txt_index))});
                        *txt_index += 1;
                    }
                },
                _ => (),
            }
        }
    }

    if loop_stack != 0 {
        panic!("loop that starts at #{} has no matching ending!", loop_start);
    }

    program
}

/// Executes a program that was previously parsed
fn run(instructions: &Vec<InstructionIndex>, tape: &mut Vec<u8>, data_pointer: &mut usize, send_cell: std::sync::mpsc::Sender<CellChange>, recieve_data: &std::sync::mpsc::Receiver<u8>) {
    for instr in instructions {
        if RESET.load(Ordering::Relaxed) {
            return;
        }
        thread::sleep(time::Duration::from_millis(10));
        match instr {
            InstructionIndex{index: i, code: Instruction::IncrementPointer} => {
                
                *data_pointer += 1;
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i});
            },
            InstructionIndex{index: i, code: Instruction::DecrementPointer} => {
                
                *data_pointer -= 1;
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i});
            },
            InstructionIndex{index: i, code: Instruction::Increment} => {
                
                tape[*data_pointer] += 1;
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i});
            },
            InstructionIndex{index: i, code: Instruction::Decrement} => {
                
                tape[*data_pointer] -= 1;
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Tape, text_index: *i});
            },
            InstructionIndex{index: i, code: Instruction::Write} => {
                
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Output, text_index: *i});
            },

            InstructionIndex{index: i, code: Instruction::Read} => {                      
                send_cell.send(CellChange{index: *data_pointer, content: tape[*data_pointer], action: Action::Input, text_index: *i});
                let input = recieve_data.recv().unwrap();
                tape[*data_pointer] = input;
            },


            InstructionIndex{index: i, code: Instruction::Loop(nested_instructions)} => {
                while tape[*data_pointer] != 0 {
                    run(&nested_instructions, tape, data_pointer, send_cell.clone(), &recieve_data)
                }
            }
        }
    }
}

//starts the parsing and visualizing
fn start_parsing(tape_lbls: &Vec<gtk::Label>, marker_lbls: &Vec<gtk::Label>, input: &gtk::TextView, output: &gtk::TextView){

    RESET.store(false, Ordering::Relaxed);

    for i in 0..32 {
        tape_lbls[i].set_text("0");
        marker_lbls[i].set_text("");
    }

    input.set_editable(false);
    input.set_cursor_visible(false);

    let mut txt_index:usize = 0;

    let buf: gtk::TextBuffer = output.get_buffer().unwrap();
    let in_buf: gtk::TextBuffer = input.get_buffer().unwrap();
    buf.set_text("");
    output.set_buffer(Some(&buf));
    let source_buffer = input.get_buffer().unwrap();
    let source = source_buffer.get_text(&source_buffer.get_start_iter(), &source_buffer.get_end_iter(), false);
    let opcodes = lex(String::from(source.as_ref().unwrap().as_str()));
    let program = parse(opcodes, &mut txt_index);
    let mut tape: Vec<u8> = vec![0; 32];
    let mut data_pointer = 0;

    let (send_cell, recieve_cell) = mpsc::channel();
    let (send_data, recieve_data) = mpsc::channel();

    let interpreter = thread::spawn(move || {

        run(&program, &mut tape, &mut data_pointer, send_cell, &recieve_data);
        
    });
    loop { 
        let received = recieve_cell.recv();

        match received.clone(){
            Ok(CellChange{index: _, content: _, action: Action::Output, text_index: i}) => {
                let mut output_txt = String::from(buf.get_text(&buf.get_start_iter(), &buf.get_end_iter(), false).unwrap().as_str());
                in_buf.select_range(&in_buf.get_iter_at_offset(i as i32), &in_buf.get_iter_at_offset(i as i32 +1));

                output_txt.push(received.unwrap().content as char);
                buf.set_text(output_txt.as_str());
                while gtk::events_pending(){
                    gtk::main_iteration();
                }
            }
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

                dialog_window.close();
                
                if (response == gtk::ResponseType::Ok) && (text != " ") {
                    send_data.send(text.as_bytes()[0]);
                }
                else {
                    println!("no input")
                }
                while gtk::events_pending(){
                    gtk::main_iteration();
                }
            },
            Err(_) => {
                println!("{:?}", received);
                break;
            }
        }       
    }
}


fn reset_app(tape_lbls: &Vec<gtk::Label>, marker_lbls: &Vec<gtk::Label>, input: &gtk::TextView, output: &gtk::TextView){

    RESET.store(true, Ordering::Relaxed);

    for i in 0..32 {
        tape_lbls[i].set_text("0");
        marker_lbls[i].set_text("");
    }

    input.set_editable(true);
    input.set_cursor_visible(true);

    let buf: gtk::TextBuffer = output.get_buffer().unwrap();
    let in_buf: gtk::TextBuffer = input.get_buffer().unwrap();
    
    buf.set_text("");
    in_buf.set_text("");

    while gtk::events_pending(){
        gtk::main_iteration();
    }
}


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

    
    start_button.connect_clicked(move |but| {

        but.set_sensitive(false);
        start_parsing(&tape_lbls, &marker_lbls, &input, &output);
        but.set_sensitive(true);
    });

    // reset_button.connect_clicked(|_|{
        
    //     reset_app(&tape_lbls, &marker_lbls, &input, &output)

    // });


    gtk::main();

    // // Lex file into opcodes
    // let opcodes = lex(source);

    // // Parse opcodes into program
    // let program = parse(opcodes);

    // // Set up environment and run program
    // let mut tape: Vec<u8> = vec![0; 1024];
    // let mut data_pointer = 512;

    // run(&program, &mut tape, &mut data_pointer);
}