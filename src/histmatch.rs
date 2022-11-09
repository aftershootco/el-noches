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

// Calculates the Cumulative Sum of the input array inplace
#[inline]
fn cumwantsome<T: Add<Output = T> + Copy + Default, const LEN: usize>(
    mut arr: [T; LEN],
) -> [T; LEN] {
    for i in 1..arr.len() {
        arr[i] = arr[i - 1] + arr[i];
    }
    arr
}

// CDF is Cumulative Distributive Frequency which is basically Cumulative Sum
// divided by the total sum.
#[inline]
fn cdf<const LEN: usize>(arr: [f32; LEN]) -> [f32; LEN] {
    let mut cdf = cumwantsome(arr);
    let number = cdf[LEN - 1];
    cdf.iter_mut().for_each(|i| {
        *i /= number;
    });
    cdf
}

// Basically creates a Lookup Table that has pixelvalue on lhs and also pixel value on rhs 
// To create this table we take the CDF array and then take the array on the left iterate 
// through it sequentially and take it's value(frequency) find the closest value in the ref_img_cdf
// then map the pixel value of current to the parent of the value found.
// eg. mapping([(1,2); (2, 10)], [(1,10); (2,2)]) => [(1,2), (2,1)]
// or  mapping([(k1,v1); (k2, v2)], [(X1,Y1); (X2,Y2)]) => [(K1,X2), (k2,X1)]
// These value were matched by looking at frequency.

fn mapping<const LEN: usize>(src_img_cdf: &[u32; LEN], ref_img_cdf: &[u32; LEN]) -> [u8; 256] {
    let lookup: BTreeMap<i64, i64> = ref_img_cdf
        .iter()
        .enumerate()
        .map(|(value, frequency)| (*frequency as i64, value as i64))
        .collect();
    let mut mapped = [0; 256];
    for (i, n) in src_img_cdf.iter().enumerate() {
        let key = *n as i64;
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

// Applies the mapped array back into image for every channel.
// For a single channel how it works is by replacing the current value 
// with it's value in the mapped array.
// For eg. if the current value is 3 we will look at the mapped_array[3] 
// and set whatever value we get from there inplace of 3.

#[inline]
fn apply(r_map: &[u8; 256], g_map: &[u8; 256], b_map: &[u8; 256], src_img: &mut [u8]) {
    src_img.chunks_exact_mut(3).for_each(|channel| {
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
