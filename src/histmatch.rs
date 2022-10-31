use std::{ops::Add, collections:: BTreeMap};

use image::{RgbImage, Rgb, GenericImageView, DynamicImage};
struct ImageChannels{
    r: Vec<u8>,
    g: Vec<u8>,
    b: Vec<u8>,
    width: u32,
    height: u32,
}

impl From<DynamicImage> for ImageChannels {
    fn from(image: DynamicImage) -> Self {
        let width = image.width();
        let height = image.height();
        let mut r: Vec<u8> = Vec::with_capacity((width*height) as usize);
        let mut g: Vec<u8> = Vec::with_capacity((width*height) as usize);
        let mut b: Vec<u8> = Vec::with_capacity((width*height) as usize);
        for i in 0..height {
            for j in 0..width {
                let pixel = image.get_pixel(j, i).0;
                r.push(pixel[0] as u8);
                g.push(pixel[1] as u8);
                b.push(pixel[2] as u8);
            }
        }
        Self {r, g, b, width, height}
    }
}
 
impl ImageChannels {
    fn get_height(&self) -> u32 { self.height }
    fn get_width(&self) -> u32 { self.width }
    fn get_channel(&self, channel: char) -> &Vec<u8>{
        match channel {
            'r' => &self.r,
            'g' => &self.g,
            'b' => &self.b,
            _ => panic!("Only 'r'/'g'/'b' channel allowed."),
        }
    }
}

struct ChannelsHistogram{
    hist: ([f32; 256], [f32; 256], [f32; 256]),
    width: u32,
    height: u32,
}

impl From<&ImageChannels> for ChannelsHistogram {
    fn from(img: &ImageChannels) -> Self {
        let mut histogram_r = [0.0; 256];
        let mut histogram_g = [0.0; 256];
        let mut histogram_b = [0.0; 256];
        let width = img.get_width();
        let height = img.get_height();
        for i in 0..height{
            for j in 0..width{
                histogram_r[(img.get_channel('r'))[(j + (i * width)) as usize] as usize] += 1.;
                histogram_g[(img.get_channel('g'))[(j + (i * width)) as usize] as usize] += 1.;
                histogram_b[(img.get_channel('b'))[(j + (i * width)) as usize] as usize] += 1.;
            }
        }
    Self{ hist: (histogram_r, histogram_b, histogram_g), width, height}
    }
}

impl ChannelsHistogram {
    fn get_channel(&self, channel: char) -> [f32; 256] {
        match channel {
            'r' => self.hist.0,
            'g' => self.hist.1,
            'b' => self.hist.2,
            _ => panic!("Only 'r'/'g'/'b' channel allowed."),
        }
    }
}

fn equalize(img: Vec<f32>) -> [u32; 256]{
    const L: u32 = 256;
    let mut new_pixel_level: [u32; 256] = [0; 256];
    for i in 0..L{
        new_pixel_level[i as usize] = ((img[i as usize] * 255.0).ceil()) as u32;
    }
    new_pixel_level
}

fn cumwantsome<T: Add<Output=T> + Copy>(arr: &[T]) -> Vec<T>{
    let mut cumsum = Vec::<T>::with_capacity(arr.len());
    cumsum.push(arr[0]);
    for i in 1..arr.len() {
        cumsum.push(cumsum[i-1] + arr[i]);
    }
    cumsum
}

fn cdf(img: &[f32; 256]) -> Vec<f32> {
    let cdf = cumwantsome(img);
    let number = cdf[cdf.len() - 1];
    let mut normalized_cdf = Vec::with_capacity(img.len());
    for i in cdf.iter() {
        let x  = *i as f32 / number as f32;
        normalized_cdf.push(x);
    };
    normalized_cdf
}

fn mapping(src_img: &[u32], ref_img: &[u32]) -> [u8; 256] {
    let mut mapped = [0;256];
    let mut lookup: BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for (n,i) in ref_img.into_iter().enumerate() {
        lookup.insert(*i, n as u32);
    }
    for (i, n) in src_img.into_iter().enumerate() {
        let key = *n;
        let upper = lookup.range(key..).next();
        let lower = lookup.range(..key).rev().next();
        let ans;
        let upper = *upper.unwrap_or((&0, &255)).1;
        let lower = *lower.unwrap_or((&0, &255)).1;
        if (upper - key) <= (lower - key) {
            ans = upper;
        } else {
            ans = lower;
        }
        mapped[i] = ans as u8;
    }
    mapped
}
fn apply(hist: &[u8], img: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(img.len());
    
    for i in 0..img.len() {
        result.push(hist[img[i as usize] as usize]);
    }
    result
}

fn combine(r: &[u8], g: &[u8], b: &[u8], height: u32, width: u32) -> Vec<[u8; 3]> {
    let mut rgb: Vec<[u8; 3]> = Vec::with_capacity((width*height) as usize);
    for i in 0..height {
        for j in 0..width {
            let r = r[(j + (i * width)) as usize];
            let g = g[(j + (i * width)) as usize];
            let b = b[(j + (i * width)) as usize];
            rgb.push([r, g, b]);
        }
    }
    rgb
}

fn match_histogram(source: &'static str, reference: &'static str, name: String) {
    let src_path = std::path::Path::new(source);
    let ref_path = std::path::Path::new(reference);
    let ref_img = image::open(ref_path).expect("File cannot be opened.");
    let src_img = image::open(src_path).expect("File cannot be opened.");

    let ref_img_channels = ImageChannels::from(ref_img);
    let src_img_channels = ImageChannels::from(src_img);

    let ref_histo = ChannelsHistogram::from(&ref_img_channels);
    let src_histo = ChannelsHistogram::from(&src_img_channels);

    let ref_cdf_r = equalize(cdf(&ref_histo.get_channel('r')));
    let ref_cdf_g = equalize(cdf(&ref_histo.get_channel('g')));
    let ref_cdf_b = equalize(cdf(&ref_histo.get_channel('b')));

    let src_cdf_r = equalize(cdf(&src_histo.get_channel('r')));
    let src_cdf_g = equalize(cdf(&src_histo.get_channel('g')));
    let src_cdf_b = equalize(cdf(&src_histo.get_channel('b')));

    let mapped_r = mapping(&src_cdf_r, &ref_cdf_r);
    let mapped_g = mapping(&src_cdf_g, &ref_cdf_g);
    let mapped_b = mapping(&src_cdf_b, &ref_cdf_b);

    let r = apply(&mapped_r, &src_img_channels.get_channel('r'));
    let g = apply(&mapped_g, &src_img_channels.get_channel('g'));
    let b = apply(&mapped_b, &src_img_channels.get_channel('b'));

    let height = src_histo.height;
    let width = src_histo.width;
    let rgb = combine(&r, &g, &b, src_histo.height, src_histo.width);
    let mut new = RgbImage::new(width, height);
    for i in 0..height {
        for j in 0..width {
            new.put_pixel(j, i, Rgb(rgb[(j + (i * width)) as usize]));
        }
    }
    new.save(name).unwrap();
}
