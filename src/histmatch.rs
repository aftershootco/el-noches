use std::{collections::BTreeMap, ops::Add};

pub struct ImageChannels<'image> {
    image: &'image mut [u8],
    width: u32,
    height: u32,
}

impl<'image> ImageChannels<'image> {
    pub fn new(image: &'image mut [u8], width: u32, height: u32) -> Self {
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

impl From<&ImageChannels<'_>> for ChannelsHistogram {
    fn from(img: &ImageChannels) -> Self {
        let mut histogram_r = [0.0; 256];
        let mut histogram_g = [0.0; 256];
        let mut histogram_b = [0.0; 256];
        let width = img.get_width();
        let height = img.get_height();
        img.image.chunks_exact(3).for_each(|channel| {
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

#[inline]
fn equalize<const LEN: usize>(img: [f32; LEN]) -> [u32; LEN] {
    let mut new_pixel_level: [u32; LEN] = [0; LEN];
    for i in 0..LEN {
        new_pixel_level[i] = ((img[i as usize] * 255.0).ceil()) as u32;
    }
    new_pixel_level
}

#[inline]
fn cumwantsome<T: Add<Output = T> + Copy + Default, const LEN: usize>(
    mut arr: [T; LEN],
) -> [T; LEN] {
    for i in 1..arr.len() {
        arr[i] = arr[i - 1] + arr[i];
    }
    arr
}

#[inline]
fn cdf<const LEN: usize>(img: [f32; LEN]) -> [f32; LEN] {
    let mut cdf = cumwantsome(img);
    let number = cdf[LEN - 1];
    cdf.iter_mut().for_each(|i| {
        *i = *i / number;
    });
    cdf
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

#[inline]
fn apply(r_map: &[u8; 256], g_map: &[u8; 256], b_map: &[u8; 256], img: &mut [u8]) {
    img.chunks_exact_mut(3).for_each(|channel| {
        channel[0] = r_map[channel[0] as usize];
        channel[1] = g_map[channel[1] as usize];
        channel[2] = b_map[channel[2] as usize];
    });
}

pub fn match_histogram_rgb_array(source: ImageChannels, reference: ImageChannels) {
    let ref_histo = ChannelsHistogram::from(&reference);
    let src_histo = ChannelsHistogram::from(&source);

    let ref_cdf_r = equalize(cdf(ref_histo.get_channel('r')));
    let ref_cdf_g = equalize(cdf(ref_histo.get_channel('g')));
    let ref_cdf_b = equalize(cdf(ref_histo.get_channel('b')));

    let src_cdf_r = equalize(cdf(src_histo.get_channel('r')));
    let src_cdf_g = equalize(cdf(src_histo.get_channel('g')));
    let src_cdf_b = equalize(cdf(src_histo.get_channel('b')));

    let mapped_r = mapping(&src_cdf_r, &ref_cdf_r);
    let mapped_g = mapping(&src_cdf_g, &ref_cdf_g);
    let mapped_b = mapping(&src_cdf_b, &ref_cdf_b);

    apply(&mapped_r, &mapped_g, &mapped_b, source.image);
}
