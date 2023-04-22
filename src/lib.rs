                extern crate serde;
#[macro_use]    extern crate serde_derive;
                extern crate rmp_serde;

use std::path::{ Path, PathBuf };
use std::io::{self, Write};
use std::fs;

use image;
use serde:: { Serialize, Deserialize };
use log:: { trace, info, warn, error };



/// To run this test, put some images (preferrably, all the same dimensions) in a local folder as defined below.
/// This function will call the `Self::new(...)` method to completely create a brand-new container format using 
/// that directory, which you can check for completeness manually.
#[test]
fn test_new() {
    
    let mut animation_container = AnimationContainer::new(&Path::new("tests/new_motionpixel_container/"));

    // The animation container must be okay.
    assert!(animation_container.is_ok());

    let mut animation_container = animation_container.unwrap();

    // Writing the animation metadata should also be okay.
    assert!(animation_container.overwrite().is_ok());

}

/// Permform a sanity check.
#[test]
fn test_sanity_check() {

    

}


/// A single animation container.
/// 
/// Assume this reflects a single animation container format. All of the frames are kept inside the animation 
/// directory, and the residual `fps`, `direction` and `looping` items are stored within the `.animation` file inside 
/// that directory. The format doesn't really care what image format the frames actually are, as long as the `image` 
/// crate *can read the format and convert it*. If you need more information about what formats are actually desired, 
/// see [this module for container formats](https://docs.rs/image/0.24.6/image/enum.ImageFormat.html) and [this module 
/// for image color values](https://docs.rs/image/0.24.6/image/enum.DynamicImage.html).
#[derive(Serialize, Deserialize)]
pub struct AnimationContainer {

    /// How many frames are shown per second? And, how dynamic do we want the frame rate to be?
    pub fps:            AnimatedFrameRate,

    /// Which direction is this animation playing?
    pub direction:      AnimatedPlayDirection,

    /// How are looping this animation (if at all?)
    pub looping:        AnimatedLoopingTechnique,

    /// A collection of the frames in this animation.
    /// 
    /// This is **not** converted or anything special on your filesystem. This is simply a collection of the files 
    /// which have been *read* from the directory of choice, and then copied into memory. The files themselves are left
    /// as-is.
    #[serde(skip_serializing, skip_deserializing)]
    pub frames:         Vec<image::DynamicImage>,

    // This is the location we took the frames from.
    working_directory:  PathBuf

}

impl AnimationContainer {

    /// Create a new format based on the contents of a directory.
    /// 
    /// It's assumed that this directory contains all of the frames of the animation in a compatible file format. 
    /// Frames that are incompatible or malformed are thrown out without warning.
    pub fn new(path: &Path) -> io::Result<Self> {

        // Create a brand-new animation container.
        let mut container = AnimationContainer {
            fps:                AnimatedFrameRate::LinearFramePerSecond(1),
            direction:          AnimatedPlayDirection::Forward,
            looping:            AnimatedLoopingTechnique::LoopInfiniteLinear,
            frames:             Vec::new(),
            working_directory:  PathBuf::from(path)
        };

        // Read the contents of the directory provided. This operation *must* succeed, or we'll return this method 
        // with `Err(...)` because no further operations can be completed.
        let files = match fs::read_dir(path) {
            Ok(dir)     => dir,
            Err(why)    => return Err(why)
        };

        // Visit every file in the directory.
        for each_file in files {
            match each_file {

                // Take the file path for correct entries.
                Ok(file_path)   => {
                    // Check and read images.
                    match image::open(&file_path.path()) {
                        
                        // Push the file to our collection.
                        Ok(this_file)   => {
                            container.frames.push(this_file)
                        },

                        Err(why)        => {
                            error!("File at {} is not readable by the image crate. Possibly a malformed file?", why)
                        }

                    }
                },

                Err(why)        => error!(
                    "File in {} doesn't seem to be accessible, probably a permissions issue?",
                    why
                )

            }
        }

        // Lastly, return our container!
        Ok(container)
        
    }

    /// Overwrite the container meta information with what we currently have.
    /// 
    /// Call this method when you don't care about overwriting meta information, or if it's your intention to do so.
    pub fn write_destructive(&self) -> io::Result<()> {
        
        let mut working_directory = self.working_directory.clone();
        working_directory.push(".animation");
        let mut file_buffer = Vec::new();
        self.serialize(&mut rmp_serde::Serializer::new(&mut file_buffer)).unwrap();

        let mut file_operation = fs::File::create(working_directory)?;
        file_operation.write_all(&file_buffer)?;

        Ok(())
        
    }

    /// Attempt to create an `AnimationContainer` from the contents of an animation container directory.
    pub fn from_directory(anim_dir: &Path) -> result::ContainerResult<Self> {
        
        // Catch paths that are not an animation container.
        if anim_dir.is_file() {
            return result::ContainerResult::Err(result::ContainerErrorKind::NotAnAnimationDirectory)
        }

        // Set us up to get the meta info for this animation.
        let mut anim_dir_meta = anim_dir.to_owned();
        anim_dir_meta.push(".animation");
        let file_operation = fs::File::open(anim_dir_meta);

        // Catch file operations not being usable because of an IO error.
        if file_operation.is_err() {
            return result::ContainerResult::Err(result::ContainerErrorKind::IoError(file_operation.unwrap_err()))
        }

        // Decode into struct.
        let file_operation = file_operation.unwrap();
        match rmp_serde::decode::from_read::<fs::File, AnimationContainer>(file_operation) {
            
            Ok(containerstruct) => {
                return result::ContainerResult::Ok(containerstruct)
            },

            Err(why)            => {
                // Catch decode issues.
                return result::ContainerResult::Err(result::ContainerErrorKind::MetadataMalformed)
            }

        }

    }
    
    // /// Ensure that the working directory for our animation container still exists and the contents of which are still 
    // /// within the lifetime of `Self` and not malformed.
    // fn sanity_check(&self) -> io::Result<()> {
        
    //     // Attempt to visit all items in the working directory and iterate over them.
    //     // We also have the *index number* (`index_no`) for every item we visit.
    //     for (index_no, each_image) in fs::read_dir(
    //         &self.working_directory
    //     )?.enumerate() {
            
    //         // Open the image in the working directory.
    //         let this_image_path = each_image?.path();
    //         let open_image = image::open(&this_image_path);

    //         // Catch malformed images.
    //         if open_image.is_err() {
    //             return Err(io::Error::new(io::ErrorKind::InvalidData, "Image data is invalid."))
    //         }

    //         // Catch frames that do not map exactly to this frame in iteration.
    //         if self.frames.get(index_no).unwrap() != &open_image.unwrap() {
    //             return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Comparator Error: Image #{} at {} does not reflect what's in memory.", index_no, this_image_path.display())))
    //         }

    //     }

    //     Ok(())

    // }
    
}

/// The frame rate type of the animation.
/// 
/// Currently, there only exists a Linear Rate. But I'm thinking we'd probably like to implement a dynamic frame rate 
/// with which to change based on a keyframe, or on-demand by the code that should call this functionality.
#[derive(Serialize, Deserialize)]
pub enum AnimatedFrameRate {

    /// Animation is frame-by-frame on this FPS value, and never changes.
    LinearFramePerSecond(i32),

}

/// Specifies the direction of the animation while playing.
#[derive(Serialize, Deserialize)]
pub enum AnimatedPlayDirection {

    Forward,
    Backward

}

/// The loop iteration technique. You may choose to linearly loop an animation (most are done this way) or with a 
/// different, even dynamic, system.
#[derive(Serialize, Deserialize)]
pub enum AnimatedLoopingTechnique {

    /// Do not end the animation, just restart it.
    LoopInfiniteLinear,

    /// Do not end the animation, just reverse it when you reach the end, and play it forward again.
    LoopInfiniteBouncing,

    /// Loop the animation based on keyframes specified.
    LoopKeyframed {
        starting:       i32,
        ending:         i32
    },

    /// Loop the animation based on the keyframes specified, while also inheriting the 'bouncing behavior' as described
    /// in the `Self::LoopInfiniteBouncing` type.
    LoopKeframedBouncing {
        starting:       i32,
        ending:         i32
    },

    /// The animation ends when the last frame is reached, with no need for looping.
    NoLoop,

}

/// Specialized types for interacting with the expected output of `AnimationContainer`.
pub mod result {

    pub enum ContainerResult<T> {

        Ok(T),
        
        Err(ContainerErrorKind)
    
    }
    
    /// Types of errors that might happen with `AnimationContainer` and the context in which you work with it.
    pub enum ContainerErrorKind {
    
        /// If you provided a path to the supposed animation container directory, it's probably *not* a directory...? 
        /// You're probably pointing at a file.
        NotAnAnimationDirectory,
    
        /// The animation directory contains new content and our object is out-of-date.
        FilesystemWasUpdatedWithoutSyncing,

        /// The contents of the animation meta file are not what we expected.
        MetadataMalformed,
    
        /// Related to an IO Error.
        IoError(std::io::Error),

        /// A serde-related encoder error.
        EncodeError(rmp_serde::encode::Error),

        /// A serde-related decoder error.
        DecodeError(rmp_serde::encode::Error),
    
    }

}
