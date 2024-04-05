use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::{Result, Context};
use tokio::{fs, fs::OpenOptions, io::{self, AsyncReadExt, AsyncWriteExt}, sync::Mutex};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerdataError
{
	#[error("OpenOptions::open failed.")]
	Open
}

pub struct Serder
{
	path: PathBuf,
}
impl Serder
{
	pub async fn new(package_name: String) -> Result<Serder>
	{
		let dirs = ProjectDirs::from("","", &package_name).context(r"Error finding data_dir: https://crates.io/crates/directories")?;
		let data_dir = dirs.data_dir();
		fs::create_dir_all(data_dir).await?;
		Ok(Serder { path: data_dir.into() })
	}
	pub async fn deserialize_or_default<T>(&self, filename: String) -> io::Result<T>
	where
		T: Default + for<'de> Deserialize<'de>
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().read(true).write(true).create(true).open(filepath).await?;

		let mut buf = Vec::new();	
		let size = file.read_to_end(&mut buf).await?;

		let output: T = match size
		{
			0 => Default::default(),
			1.. => serde_json::from_slice(&buf)?,
		};

		Ok(output)
	}
	pub async fn deserialize_or_err<T>(&self, filename: String) -> Result<T>
	where
		T: Default + for<'de> Deserialize<'de>
	{
		let buf = self.read_file(filename).await?;
	
		let output = match buf.len()
		{
			0 => None,
			1.. => Some(serde_json::from_slice(&buf)?),
		}.context("File empty.")?;
		Ok(output)
	}
	pub async fn serialize_and_save<T>(&self, filename: String, data: T) -> io::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		let serialized_data = serde_json::to_string(&data)?;
		file.write_all(serialized_data.as_bytes()).await?;
		Ok(())
	}
	pub async fn serialize_arc_and_save<T>(&self, filename: String, data: Arc<T>) -> io::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		let serialized_data = serde_json::to_string(data.as_ref())?;
		file.write_all(serialized_data.as_bytes()).await?;
		Ok(())
	}
	pub async fn serialize_arc_mutex_and_save<T>(&self, filename: String, data: Arc<Mutex<T>>) -> io::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		let locked_data = data.lock().await;
		let serialized_data = serde_json::to_string(&*locked_data)?;
		file.write_all(serialized_data.as_bytes()).await?;
		Ok(())
	}
	async fn read_file(&self, filename: String) -> Result<Vec<u8>>
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().read(true).write(true).create(true).open(filepath).await?;

		let mut buf = Vec::new();	
		file.read_to_end(&mut buf).await?;

		Ok(buf)
	}
}

#[cfg(test)]
mod tests
{
	use super::*;
	#[derive(Clone, Serialize, Deserialize, Default)] 
	pub struct T
	{
		data: E,
	}
	#[derive(Clone, Serialize, Deserialize, Default)] 
	pub enum E
	{
		#[default]
		A,
		B,
		C
	}
	#[tokio::test]
	async fn no_err() -> Result<()>
	{
		let se = Serder::new("serdata_test".to_string()).await?;
		let data: E = se.deserialize_or_default("in.json".to_string()).await?;
		se.serialize_and_save("out.json".to_string(), data).await?;

		Ok(())
	}
}
