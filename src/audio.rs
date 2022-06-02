use rodio::Sink;
use std::fs::File;
use std::io::BufReader;

pub struct Audio {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
    sink: Option<Sink>,
}

impl Audio {
    pub fn new() -> Audio {
        let (_stream, handle) = rodio::OutputStream::try_default().unwrap(); 
        let mut a = Audio {
            handle: handle,
            _stream: _stream,
            sink: None,
        };
        // nice, code that's safer in C++
        a.sink = Some(Sink::try_new(&a.handle).unwrap());

        return a;
    }

    pub fn play(&self) {
        let file = File::open("resources/out.mp3").unwrap();
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

        self.sink.as_ref().unwrap().append(source);
        // this might just immediately play, we'll see
    }

    pub fn play_file(&self, filename: &String) {
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
