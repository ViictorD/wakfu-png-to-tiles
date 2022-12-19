use image::{codecs::png::PngDecoder, DynamicImage, imageops, RgbaImage};
use std::{fs::{File, self}, path::{PathBuf}, collections::HashMap, str::FromStr, io::Read};
use anyhow::Result;

fn get_u32_from_two_u16(nb1: u16, nb2: u16) -> u32 {
	((nb1 as u32) << 16) | (nb2 as u32)
}

fn get_zoom(nb: &u8) -> u8 {
	if *nb < 3 {
		return nb - 1;
	}
	let mut count = 2u8;
	let mut min = 2u16;
	let mut max = 5u16;
	while max < *nb as u16 {
		min *= 2;
		max = min * 2 + 1;
		count += 1;
	}
	count
}

// Return min and max zoom level
fn calculate_min_max_zoom(size: u32) -> (u8, u8) {
	let mut nb_squares = Vec::new();
	let mut nb_square = (size as f32 / 256.).ceil();
	while nb_square > 4. {
		nb_squares.push(nb_square as u8);
		nb_square = (nb_square / 2.).ceil();
	}

	if nb_squares.len() == 0 {
		return (0, 0);
	}

	let min_zoom = get_zoom(nb_squares.get(nb_squares.len() - 1).unwrap());
	let max_zoom = get_zoom(nb_squares.get(0).unwrap());

	(min_zoom, max_zoom)
}

fn read_u32_from_array(array: [u8; 4]) -> u32 {
	(array[0] as u32) << 24 | (array[1] as u32) << 16 | (array[2] as u32) << 8 | (array[3] as u32)
}

fn get_png_width_height(path: PathBuf) -> (u32, u32) {
	let mut buffer = [0u8; 24];
	File::open(path).unwrap().read_exact(&mut buffer).unwrap();
	let width = read_u32_from_array([buffer[16], buffer[17], buffer[18], buffer[19]]);
	let height = read_u32_from_array([buffer[20], buffer[21], buffer[22], buffer[23]]);
	(width, height)
}

fn read_sizes(path: PathBuf) -> (u32, u32) {
	let mut width: u32 = 0;
	let mut height: u32 = 0;
	let mut read_x = vec![];
	let mut read_y = vec![];
	for dir in fs::read_dir(path).unwrap() {
		let file_path = dir.unwrap().path();
		let filename = file_path.file_stem().unwrap().to_str().unwrap();
		if filename.contains("_") {
			let splited_filename = filename.split("_").collect::<Vec<&str>>();
			let x = splited_filename.get(1).unwrap().parse::<u8>().unwrap();
			let y = splited_filename.get(2).unwrap().parse::<u8>().unwrap();
			let (w, h) = get_png_width_height(file_path);
			if !read_x.contains(&x) {
				read_x.push(x);
				width += w;
			}
			if !read_y.contains(&y) {
				read_y.push(y);
				height += h;
			}
		}
		else {
			return get_png_width_height(file_path);
		}
	}

	(width, height)
}

fn run(path: PathBuf, output_path: PathBuf, total_width: u32, total_height: u32) -> Result<()> {
	let nb_zoom_width = calculate_min_max_zoom(total_width);
	let nb_zoom_height = calculate_min_max_zoom(total_height);

	let zoom =
		if nb_zoom_width.1 == nb_zoom_height.1 {
			if nb_zoom_width.0 < nb_zoom_height.0 { nb_zoom_width } else { nb_zoom_height }
		}
		else {
			if nb_zoom_width.1 > nb_zoom_height.1 { nb_zoom_width } else { nb_zoom_height }
		};

	let max_zoom_dir = output_path.join(format!("{}", zoom.1));
	if !max_zoom_dir.exists() {
		fs::create_dir_all(max_zoom_dir.clone())?;
	}
	for i in (zoom.0..zoom.1).rev() {
		let output_dir = output_path.join(format!("{i}"));
		if !output_dir.exists() {
			fs::create_dir_all(output_dir.clone())?;
		}
	}

	for dir in fs::read_dir(path).unwrap() {
		let file_path = dir.unwrap().path();
		println!("Processing image: {:?}...", file_path.to_str().unwrap());
		let image = DynamicImage::from_decoder(
			PngDecoder::new(File::open(file_path.clone())?)?
		).unwrap().into_rgba8();
		let filename = file_path.file_stem().unwrap().to_str().unwrap();
		let mut png_x: u16;
		let mut png_y: u16;
		if filename.contains("_") {
			let splited_filename = filename.split("_").collect::<Vec<&str>>();
			png_x = splited_filename.get(1).unwrap().parse::<u16>().unwrap() * 256;
			png_y = splited_filename.get(2).unwrap().parse::<u16>().unwrap() * 256;
		}
		else {
			png_x = 0;
			png_y = 0;
		}

		let mut tiles = HashMap::new();
		let mut tiles_2 = HashMap::new();

		let mut x_tiles = (image.width() as f32 / 256.).ceil() as u16;
		let mut y_tiles = (image.height() as f32 / 256.).ceil() as u16;

		// Split the hole image into 256x256 tiles
		println!("Processing zoom: {}...", zoom.1);
		for x in 0..x_tiles {
			for y in 0..y_tiles {
				let mut output = imageops::crop_imm(
					&image,
					x as u32 * 256,
					y as u32 * 256,
					256,
					256
				).to_image();
				if x + 1 == x_tiles || y + 1 == y_tiles {
					let mut tmp = RgbaImage::new(256, 256);
					imageops::replace(&mut tmp, &output, 0, 0);
					output = tmp;
				}

				output.save_with_format(format!("{}/{}_{}.png", max_zoom_dir.as_os_str().to_str().unwrap(), png_x + x, png_y + y), image::ImageFormat::Png)?;
				tiles.insert(get_u32_from_two_u16(x, y), output);
			}
		}

		drop(image);

		for i in (zoom.0..zoom.1).rev() {
			let output_dir = output_path.join(format!("{i}"));
			println!("Processing zoom: {:?}...", i);
			for x in (0..x_tiles).step_by(2) {
				for y in (0..y_tiles).step_by(2) {
					let top_left = tiles.get(&get_u32_from_two_u16(x, y)).unwrap();
					let (top_right, bottom_left, bottom_right) =
						if x + 1 == x_tiles || y + 1 == y_tiles {
							if x + 1 == x_tiles && y + 1 == y_tiles {
								let top_right = RgbaImage::new(256, 256);
								let bottom_right = RgbaImage::new(256, 256);
								let bottom_left = RgbaImage::new(256, 256);
								(top_right, bottom_left, bottom_right)
							}
							else if x + 1 == x_tiles {
								let top_right = RgbaImage::new(256, 256);
								let bottom_left = tiles.get(&get_u32_from_two_u16(x, y + 1)).unwrap().clone();
								let bottom_right = RgbaImage::new(256, 256);
								(top_right, bottom_left, bottom_right)
							}
							else {
								let top_right = tiles.get(&get_u32_from_two_u16(x + 1, y)).unwrap().clone();
								let bottom_left = RgbaImage::new(256, 256);
								let bottom_right = RgbaImage::new(256, 256);
								(top_right, bottom_left, bottom_right)
							}
						}
						else {
							let top_right = tiles.get(&get_u32_from_two_u16(x + 1, y)).unwrap().clone();
							let bottom_left = tiles.get(&get_u32_from_two_u16(x, y + 1)).unwrap().clone();
							let bottom_right = tiles.get(&get_u32_from_two_u16(x + 1, y + 1)).unwrap().clone();
							(top_right, bottom_left, bottom_right)
						};

					let mut output = RgbaImage::new(512, 512);
					imageops::replace(&mut output, top_left, 0, 0);
					imageops::replace(&mut output, &top_right, 256, 0);
					imageops::replace(&mut output, &bottom_left, 0, 256);
					imageops::replace(&mut output, &bottom_right, 256, 256);

					output = imageops::resize(&output, 256, 256, imageops::FilterType::CatmullRom);

					let output_x = ((png_x + x) as f32 / 2.).ceil() as u16;
					let output_y = ((png_y + y) as f32 / 2.).ceil() as u16;

					let tile_path = PathBuf::from(format!("{}/{output_x}_{output_y}.png", output_dir.as_os_str().to_str().unwrap()));
					if tile_path.exists() {
						println!("[WARNING] Replacing an other tile: {:?}", tile_path);
					}

					output.save_with_format(tile_path, image::ImageFormat::Png)?;
					tiles_2.insert(get_u32_from_two_u16(x / 2, y / 2), output);
				}
			}
			tiles = tiles_2;
			tiles_2 = HashMap::new();
			x_tiles = (x_tiles as f32 / 2.).ceil() as u16;
			y_tiles = (y_tiles as f32 / 2.).ceil() as u16;
			png_x = (png_x as f32 / 2.).ceil() as u16;
			png_y = (png_y as f32 / 2.).ceil() as u16;
		}
	}
	Ok(())
}

fn main() -> Result<()> {
	let input_path = PathBuf::from("./input");
	for dir in fs::read_dir(input_path)? {
		let sub_path = dir?.path();
		let outdoor_path = sub_path.join("outdoor");
		let indoor_path = sub_path.join("indoor");
		if outdoor_path.exists() {
			let (width, height) = read_sizes(outdoor_path.clone());
			let output_dir = PathBuf::from_str(
				outdoor_path.to_str().unwrap().to_string().replace("input", "output").as_str()
			)?;
			run(outdoor_path.clone(), output_dir, width, height)?;
		}
		if indoor_path.exists() {
			let (width, height) = read_sizes(outdoor_path.clone());
			let output_dir = PathBuf::from_str(
				indoor_path.to_str().unwrap().to_string().replace("input", "output").as_str()
			)?;
			run(indoor_path.clone(), output_dir, width, height)?;
		}
	}
	Ok(())
}
