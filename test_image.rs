use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use image::DynamicImage;

fn test() {
    let mut picker = Picker::from_termios().unwrap();
    let img = image::DynamicImage::new_rgb8(10, 10);
    let protocol: Box<dyn StatefulProtocol> = picker.new_resize_protocol(img);
}
