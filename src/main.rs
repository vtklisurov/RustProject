extern crate gtk;
extern crate gio;


use gtk::prelude::*;
use gio::prelude::*;

use gtk::{Application, ApplicationWindow, Button};

use std::env;
use std::io::Read;
use std::sync::mpsc;
use std::{thread, time};

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
                    println!("increment pointer");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::IncrementPointer})                 
                },
                OpCode::DecrementPointer => {
                    println!("decrement pointer");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::DecrementPointer})
                },
                OpCode::Increment => {
                    println!("increment data");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Increment})
                },
                OpCode::Decrement => {
                    println!("decrement data");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Decrement})
                },
                OpCode::Write => {
                    println!("write");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Write})
                },
                OpCode::Read => {
                    println!("read");
                    println!("{:?}", *txt_index);
                    *txt_index += 1;
                    Some(InstructionIndex{index: *txt_index-1, code: Instruction::Read})
                },

                OpCode::LoopBegin => {
                    println!("loop begin");
                    println!("{:?}", *txt_index);
                    loop_start = i;
                    loop_stack += 1;
                    None
                },

                OpCode::LoopEnd => {
                    println!("error loop");
                    println!("{:?}", *txt_index);
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
                    println!("nested loop begin");
                    println!("{:?}", *txt_index);
                    loop_stack += 1;
                },
                OpCode::LoopEnd => {
                    println!("nested loop end");
                    println!("{:?}", *txt_index);

                    loop_stack -= 1;

                    if loop_stack == 0 {
                        println!("loop end");
                        println!("{:?}", *txt_index);
                        *txt_index += 1;
                        program.push(InstructionIndex{index: i, code: Instruction::Loop(parse(opcodes[loop_start+1..i].to_vec(), txt_index))});
                    }
                },
                _ => {
                    *txt_index-=1;
                },
            }
        }
    }

    if loop_stack != 0 {
        panic!("loop that starts at #{} has no matching ending!", loop_start);
    }

    program
}

/// Executes a program that was previously parsed
fn run(instructions: &Vec<InstructionIndex>, tape: &mut Vec<u8>, data_pointer: &mut usize, send_cell: std::sync::mpsc::Sender<CellChange>) {
    for instr in instructions {
        thread::sleep(time::Duration::from_millis(100));
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

            //FIX THIS ------------------------------------------------------------------------------------------------------------------------------//
            InstructionIndex{index: i, code: Instruction::Read} => {
                
                todo!();
                let mut input: [u8; 1] = [0; 1];
                std::io::stdin().read_exact(&mut input).expect("failed to read stdin");
                tape[*data_pointer] = input[0];
            },
            //---------------------------------------------------------------------------------------------------------------------------------------//
            InstructionIndex{index: i, code: Instruction::Loop(nested_instructions)} => {
                
                while tape[*data_pointer] != 0 {
                    println!("looping");
                    run(&nested_instructions, tape, data_pointer, send_cell.clone())
                }
            }
        }
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

    
    start_button.connect_clicked(move |_| {
        for i in 0..32 {
            tape_lbls[i].set_text("0");
            marker_lbls[i].set_text("");
        }

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

        let interpreter = thread::spawn(move || {

            run(&program, &mut tape, &mut data_pointer, send_cell);
            
        });
        loop { 
            let received = recieve_cell.recv();
            // let receivedOut = recieveOut.recv();
            // match receivedOut{
            //     Ok(_) =>{
            //         let mut tmp = buf.get_text(&buf.get_start_iter(), &buf.get_end_iter(), false);
            //         let mut output = String::from(tmp.unwrap().as_str());
            //         output.push(receivedOut.unwrap());
            //         buf.set_text(output.as_str());
            //         while gtk::events_pending(){
            //             gtk::main_iteration();
            //         }
            //     },
            //     Err(_) => {
            //         println!("{:?}", receivedOut);
            //         break;
            //     }
            // }
            match received.clone(){
                Ok(CellChange{index: _, content: _, action: Action::Output, text_index: i}) => {
                    println!("{:?}", received);
                    let mut output = String::from(buf.get_text(&buf.get_start_iter(), &buf.get_end_iter(), false).unwrap().as_str());
                    in_buf.select_range(&in_buf.get_iter_at_offset(i as i32), &in_buf.get_iter_at_offset(i as i32 +1));

                    output.push(received.unwrap().content as char);
                    buf.set_text(output.as_str());
                    while gtk::events_pending(){
                        gtk::main_iteration();
                    }
                }
                Ok(CellChange{index: _, content: _, action: Action::Tape, text_index: i}) => {
                    println!("{:?}", received);
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
                Err(_) => {
                    println!("{:?}", received);
                    break;
                }
            }       
        }

        pause_button.connect_clicked(move |_| {


        });
    
    });




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