extern crate gtk;
extern crate gio;

use gtk::prelude::*;
use gio::prelude::*;

use gtk::{Application, ApplicationWindow, Button};

use std::env;
use std::io::Read;
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
    Loop(Vec<Instruction>)
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

fn parse(opcodes: Vec<OpCode>) -> Vec<Instruction> {
    let mut program: Vec<Instruction> = Vec::new();
    let mut loop_stack = 0;
    let mut loop_start = 0;

    for (i, op) in opcodes.iter().enumerate() {
        if loop_stack == 0 {
            let instr = match op {
                OpCode::IncrementPointer => Some(Instruction::IncrementPointer),
                OpCode::DecrementPointer => Some(Instruction::DecrementPointer),
                OpCode::Increment => Some(Instruction::Increment),
                OpCode::Decrement => Some(Instruction::Decrement),
                OpCode::Write => Some(Instruction::Write),
                OpCode::Read => Some(Instruction::Read),

                OpCode::LoopBegin => {
                    loop_start = i;
                    loop_stack += 1;
                    None
                },

                OpCode::LoopEnd => panic!("loop ending at #{} has no beginning", i),
            };

            match instr {
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
                        program.push(Instruction::Loop(parse(opcodes[loop_start+1..i].to_vec())));
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
fn run(instructions: &Vec<Instruction>, tape: &mut Vec<u8>, data_pointer: &mut usize, text_buffer: &mut gtk::TextBuffer) {
    for instr in instructions {
        while gtk::events_pending(){
            gtk::main_iteration();
        }
        match instr {
            Instruction::IncrementPointer => *data_pointer += 1,
            Instruction::DecrementPointer => *data_pointer -= 1,
            Instruction::Increment => tape[*data_pointer] += 1,
            Instruction::Decrement => tape[*data_pointer] -= 1,
            Instruction::Write => {
                thread::sleep(time::Duration::from_millis(100));
                let mut tmp = text_buffer.get_text(&text_buffer.get_start_iter(), &text_buffer.get_end_iter(), false);
                let mut output = String::from(tmp.unwrap().as_str());
                output.push(tape[*data_pointer] as char);
                text_buffer.set_text(output.as_str());
            },

            //FIX THIS ------------------------------------------------------------------------------------------------------------------------------//
            Instruction::Read => {
                todo!();
                let mut input: [u8; 1] = [0; 1];
                std::io::stdin().read_exact(&mut input).expect("failed to read stdin");
                tape[*data_pointer] = input[0];
            },
            //---------------------------------------------------------------------------------------------------------------------------------------//
            Instruction::Loop(nested_instructions) => {
                while tape[*data_pointer] != 0 {
                    run(&nested_instructions, tape, data_pointer, text_buffer)
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
    let mut start_button: gtk::Button= builder.get_object("btnStart").unwrap();
    let mut pause_button: gtk::Button = builder.get_object("btnPause").unwrap();
    let mut reset_button: gtk::Button = builder.get_object("btnReset").unwrap();
    let mut speed_slider: gtk::Scale = builder.get_object("sliderSpeed").unwrap();
    let mut input: gtk::TextView = builder.get_object("txtInput").unwrap();
    let mut output: gtk::TextView = builder.get_object("txtOutput").unwrap();
    let mut tape: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
    let mut marker: Vec<gtk::Label> = vec![gtk::Label::new(None); 32];
    for i in 0..32 {
        tape[i] = builder.get_object(("lbl".to_owned() + i.to_string().as_str()).as_str()).unwrap();
        marker[i] = builder.get_object(("lblMarker".to_owned() + i.to_string().as_str()).as_str()).unwrap();
    }

    window.set_title("Brainfuck Visualizer");

    window.show_all();


    
    start_button.connect_clicked(move |_| {
        let mut buf: gtk::TextBuffer = output.get_buffer().unwrap();
        output.set_buffer(Some(&buf));
        let sourceBuffer = input.get_buffer().unwrap();
        let source = sourceBuffer.get_text(&sourceBuffer.get_start_iter(), &sourceBuffer.get_end_iter(), false);
        let opcodes = lex(String::from(source.as_ref().unwrap().as_str()));
        let program = parse(opcodes);
        let mut tape: Vec<u8> = vec![0; 32];
        let mut data_pointer = 0;


        run(&program, &mut tape, &mut data_pointer, &mut buf);
        

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