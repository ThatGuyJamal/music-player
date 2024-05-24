use std::{
    io::{BufReader, Seek},
    time::Duration,
};

use rodio::{Decoder, OutputStream, Sink};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerError {
    UnableToCloneFileHandle,
    NoFileHandle,
    NoSeekIndex,
    NotAbleToSeek,
    UnableToCreateSeekIndex,
    UnableToGetDuration,
    UnableToOpenFile,
}

pub type PlayerResult<T> = std::result::Result<T, PlayerError>;

const MAX_FILE_SIZE_FOR_SEEK_INDEX: u64 = 1024 * 1024 * 50; // 50 MB

/// A player manages audio functions for a file. Things like play, pause, resume, seek, volume.
pub struct Player {
    sink: Sink,
    _stream: OutputStream,
    file_handle: Option<std::fs::File>,
    duration: Option<Duration>,
    seek_index: Option<Vec<(Duration, u64)>>,
}
impl Player {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        Self {
            sink,
            _stream,
            file_handle: None,
            duration: None,
            seek_index: None,
        }
    }

    pub fn is_file_loaded(&self) -> bool {
        self.file_handle.is_some()
    }

    pub async fn load_path(&mut self, path: &str) -> PlayerResult<()> {
        let file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(_) => return Err(PlayerError::UnableToOpenFile),
        };

        self.load_file(file).await?;

        Ok(())
    }

    pub async fn load_file(&mut self, file: tokio::fs::File) -> PlayerResult<()> {
        self.stop();

        let reader = tokio::io::BufReader::new(file.try_clone().await.unwrap());
        let mut analyzer = vpr_audio_analyzer::Analyzer::new(reader);

        self.duration = match analyzer.get_duration().await {
            Ok(duration) => Some(duration),
            Err(_) => None,
        };

        // If the file is too big, we don't want to create a seek index
        // because it would take too long.
        let file_size = file.metadata().await.unwrap().len();
        if file_size <= MAX_FILE_SIZE_FOR_SEEK_INDEX {
            self.seek_index = match analyzer.get_seek_index().await {
                Ok(seek_index) => Some(seek_index),
                Err(_) => None,
            };
        }

        let mut std_file = file.into_std().await;
        self.file_handle = match std_file.try_clone() {
            Ok(std_file_handle) => Some(std_file_handle),
            Err(_) => return Err(PlayerError::UnableToCloneFileHandle),
        };

        std_file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let reader = BufReader::new(std_file);
        let source = Decoder::new(reader).unwrap();

        self.sink.append(source);

        Ok(())
    }

    fn get_std_file_handle(&self) -> PlayerResult<&std::fs::File> {
        match self.file_handle.as_ref() {
            Some(std_file_handle) => Ok(std_file_handle),
            None => return Err(PlayerError::NoFileHandle),
        }
    }

    fn get_bytes_offset_for_time(&self, time: Duration) -> PlayerResult<u64> {
        let seek_index = match self.seek_index.as_ref() {
            Some(seek_index) => seek_index,
            None => return Err(PlayerError::NoSeekIndex),
        };

        let mut offset = 0;
        for (frame_time, frame_offset) in seek_index {
            if frame_time > &time {
                break;
            }
            offset = *frame_offset;
        }

        Ok(offset)
    }

    fn get_time_for_bytes_offset(&self, offset: u64) -> PlayerResult<Duration> {
        let seek_index = match self.seek_index.as_ref() {
            Some(seek_index) => seek_index,
            None => return Err(PlayerError::NoSeekIndex),
        };

        let mut time = Duration::from_secs(0);
        for (frame_time, frame_offset) in seek_index {
            if frame_offset > &offset {
                break;
            }
            time = *frame_time;
        }

        Ok(time)
    }

    pub fn is_seekable(&self) -> bool {
        self.is_file_loaded() && self.seek_index.is_some()
    }

    pub fn seek(&mut self, time_offset: Duration) -> PlayerResult<()> {
        let bytes_offset = self.get_bytes_offset_for_time(time_offset)?;
        let mut std_file_handle = self.get_std_file_handle()?;

        std_file_handle
            .seek(std::io::SeekFrom::Start(bytes_offset))
            .map_err(|_| PlayerError::NotAbleToSeek)?;

        Ok(())
    }

    pub fn play(&mut self) {
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn stop(&mut self) {
        self.duration = None;
        self.seek_index = None;
        self.file_handle = None;
        self.sink.stop();
    }

    pub fn elapsed(&self) -> PlayerResult<Duration> {
        let mut file_handle = self.get_std_file_handle()?;
        let cursor_position = file_handle
            .seek(std::io::SeekFrom::Current(0))
            .map_err(|_| PlayerError::NotAbleToSeek)?;

        let elapsed_time = self.get_time_for_bytes_offset(cursor_position)?;
        Ok(elapsed_time)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume);
    }
}
