use std::{collections::BTreeMap, ops::Add};

pub struct ImageChannels {
    image: Vec<u8>,
    width: u32,
    height: u32,
}

impl ImageChannels {
    pub fn new(image: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            image,
            width,
            height,
        }
    }
    fn get_height(&self) -> u32 {
        self.height
    }
    fn get_width(&self) -> u32 {
        self.width
    }
}

#[allow(dead_code)]
struct ChannelsHistogram {
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
        let _ = img.image.as_slice().chunks_exact(3).map(|channel| {
            histogram_r[channel[0] as usize] += 1.;
            histogram_g[channel[1] as usize] += 1.;
            histogram_b[channel[2] as usize] += 1.;
        });
        Self {
            hist: (histogram_r, histogram_b, histogram_g),
            width,
            height,
        }
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

fn equalize<const LEN: usize>(img: [f32; LEN]) -> [u32; LEN] {
    let mut new_pixel_level: [u32; LEN] = [0; LEN];
    for i in 0..LEN {
        new_pixel_level[i] = ((img[i as usize] * 255.0).ceil()) as u32;
    }
    new_pixel_level
}

fn cumwantsome<T: Add<Output = T> + Copy + Default, const LEN: usize>(arr: &[T; LEN]) -> [T; LEN] {
    let mut cumsum = [T::default(); LEN];
    cumsum[0] = arr[0];
    for i in 1..arr.len() {
        cumsum[i] = cumsum[i - 1] + arr[i];
    }
    cumsum
}

fn cdf<const LEN: usize>(img: &[f32; LEN]) -> [f32; LEN] {
    let cdf = cumwantsome(img);
    let number = cdf[cdf.len() - 1];
    let mut normalized_cdf = [0.0; LEN];
    for (index, i) in cdf.iter().enumerate() {
        normalized_cdf[index] = *i / number;
    }
    normalized_cdf
}

fn mapping<const LEN: usize>(src_img: &[u32; LEN], ref_img: &[u32; LEN]) -> [u8; 256] {
    let lookup: BTreeMap<u32, u32> = ref_img
        .into_iter()
        .enumerate()
        .map(|(n, i)| (*i, n as u32))
        .collect();
    let mut mapped = [0; 256];
    for (i, n) in src_img.iter().enumerate() {
        let key = *n;
        let upper = lookup.range(key..).next();
        let lower = lookup.range(..key).rev().next();
        let upper = *upper.unwrap_or((&0, &255)).1;
        let lower = *lower.unwrap_or((&0, &255)).1;
        let ans = if (upper - key) <= (lower - key) {
            upper
        } else {
            lower
        };
        mapped[i] = ans as u8;
    }
    mapped
}
fn apply(r_map: &[u8; 256], g_map: &[u8; 256], b_map: &[u8; 256], img: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(img.len());
    for i in 0..img.len() {
        result.push(r_map[img[i as usize] as usize]);
        result.push(g_map[img[i as usize] as usize]);
        result.push(b_map[img[i as usize] as usize]);
    }
    result
}

pub fn match_histogram_rgb_array(source: ImageChannels, reference: ImageChannels) -> Vec<u8> {
    let ref_histo = ChannelsHistogram::from(&reference);
    let src_histo = ChannelsHistogram::from(&source);

    let ref_cdf_r = equalize(cdf(&ref_histo.get_channel('r')));
    let ref_cdf_g = equalize(cdf(&ref_histo.get_channel('g')));
    let ref_cdf_b = equalize(cdf(&ref_histo.get_channel('b')));

    let src_cdf_r = equalize(cdf(&src_histo.get_channel('r')));
    let src_cdf_g = equalize(cdf(&src_histo.get_channel('g')));
    let src_cdf_b = equalize(cdf(&src_histo.get_channel('b')));

    let mapped_r = mapping(&src_cdf_r, &ref_cdf_r);
    let mapped_g = mapping(&src_cdf_g, &ref_cdf_g);
    let mapped_b = mapping(&src_cdf_b, &ref_cdf_b);

    let r = apply(&mapped_r, &mapped_g, &mapped_b, &source.image);
    r
}
