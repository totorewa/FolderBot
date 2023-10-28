use rodio::{Sink, Source};
use std::fs::File;
use std::io::BufReader;

use rodio::cpal;
use rodio::cpal::traits::HostTrait;
use rodio::DeviceTrait;

pub struct Audio {
    sink: Sink,
    default_sink: Option<Sink>,
}

impl Audio {
    pub fn new() -> Audio {
        let host = cpal::default_host();
        let mut devices = host.output_devices().unwrap();

        println!("Loading audio devices...");
        let aux = devices.find(|device| {
            if let Ok(name) = device.name() {
                name.starts_with("VoiceMeeter Aux Input")
            } else {
                false
            }
        });

        if let Some(aux) = aux {
            println!("Using VoiceMeeter Aux Input for soundboard.");
            // Get our default output device also.
            let (_, dhandle) = rodio::OutputStream::try_default().unwrap();
            let (_, handle) = rodio::OutputStream::try_from_device(&aux.into()).unwrap();
            Audio {
                sink: Sink::try_new(&handle).unwrap(),
                default_sink: Sink::try_new(&dhandle).ok(),
            }
        } else {
            println!("Warning: Did not find correct output device, using default.");
            let (_, handle) = rodio::OutputStream::try_default().unwrap();
            Audio {
                sink: Sink::try_new(&handle).unwrap(),
                default_sink: None,
            }
        }
    }

    pub fn play(&self) {
        let file = File::open("resources/out.mp3").unwrap();
        let source = rodio::Decoder::new(BufReader::new(file))
            .unwrap()
            .buffered();

        // just plays immediately, it's multithreaded :)
        self.sink.append(source.clone());
        if let Some(s) = self.default_sink.as_ref() {
            s.append(source);
        }
    }

    pub fn play_file(&self, filename: &String) {
        if self.sink.len() > 2 {
            return;
        }
        let file = match File::open(filename) {
            Ok(x) => x,
            Err(_) => return, // this should print
        };
        let source = rodio::Decoder::new(BufReader::new(file))
            .unwrap()
            .buffered();

        self.sink.append(source.clone());
        if let Some(s) = self.default_sink.as_ref() {
            s.append(source);
        }
    }

    pub fn stop(&self) {
        self.sink.stop();
    }
}
