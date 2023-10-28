use rodio::Sink;
use std::fs::File;
use std::io::BufReader;

use rodio::cpal;
use rodio::cpal::traits::HostTrait;
use rodio::DeviceTrait;

pub struct Audio {
    _stream: rodio::OutputStream,
    _default_stream: Option<rodio::OutputStream>,
    sink: Sink,
    default_sink: Option<Sink>
}

impl Audio {
    pub fn new() -> Audio {
        let host = cpal::default_host();
        let mut devices = host.output_devices().unwrap();

        println!("Loading audio devices...");
        let aux = devices.find(|device| {
            if let Ok(name) = device.name() {
                name.starts_with("VoiceMeeter Aux Input")
            }
            else { false }
        });

        let ((_stream, handle), (dstream, dhandle)) = if let Some(aux) = aux {
            println!("Using VoiceMeeter Aux Input for soundboard.");
            (rodio::OutputStream::try_from_device(&aux.into()).unwrap(), rodio::OutputStream::try_default().unwrap())
        }
        else {
            println!("Warning: Did not find correct output device, using default.");
            (rodio::OutputStream::try_default().unwrap(), (None, None))
        };

        Audio {
            _stream: _stream,
            sink: Sink::try_new(&handle).unwrap(),
            handle: handle,
        }
    }

    pub fn play(&self) {
        let file = File::open("resources/out.mp3").unwrap();
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

        self.sink.as_ref().unwrap().append(source);
        // this might just immediately play, we'll see
    }

    pub fn play_file(&self, filename: &String) {
        if self.sink.as_ref().unwrap().len() > 2 {
            return;
        }
        let file = match File::open(filename) {
            Ok(x) => x,
            Err(_) => return // this should print
        };
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

        self.sink.as_ref().unwrap().append(source);
        // this might just immediately play, we'll see
    }

    pub fn stop(&self) {
        self.sink.as_ref().unwrap().stop();
    }
}
