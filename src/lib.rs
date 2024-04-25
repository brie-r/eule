use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::{Result, Context};
use tokio::{fs, fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}, sync::Mutex};
use std::sync::Arc;
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
	pub async fn deserialize_or_default<T>(&self, filename: String) -> anyhow::Result<T>
	where
		T: Default + for<'de> Deserialize<'de>
	{
		let buf = self.read_file(filename).await?;
		let output = match buf.len()
		{
			0 => Default::default(),
			1.. => ron::de::from_bytes(&buf).unwrap_or_default(),
		};

		Ok(output)
	}
	pub async fn deserialize_or_value<T>(&self, filename: String, value: T) -> anyhow::Result<T>
	where
		T: for<'de> Deserialize<'de>
	{
		let buf = self.read_file(filename).await?;
		let output = match buf.len()
		{
			0 => value,
			1.. => ron::de::from_bytes(&buf).unwrap_or(value),
		};

		Ok(output)
	}
	pub async fn deserialize_or_err<T>(&self, filename: String) -> Result<T>
	where
		T: for<'de> Deserialize<'de>
	{
		let buf = self.read_file(filename).await?;
		let output = match buf.len()
		{
			0 => None,
			1.. => Some(ron::de::from_bytes(&buf)?),
		}.context("File empty.")?;
		Ok(output)
	}
	pub async fn serialize_and_save<T>(&self, filename: String, data: T) -> anyhow::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		// TODO: save pretty
		let serialized_data = ron::ser::to_string(&data)?;
		file.write_all(serialized_data.as_bytes()).await?;
		Ok(())
	}
	pub async fn serialize_arc_and_save<T>(&self, filename: String, data: Arc<T>) -> anyhow::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		let serialized_data = ron::ser::to_string(data.as_ref())?;
		file.write_all(serialized_data.as_bytes()).await?;
		Ok(())
	}
	pub async fn serialize_arc_mutex_and_save<T>(&self, filename: String, data: Arc<Mutex<T>>) -> anyhow::Result<()>
	where
		T: Serialize
	{
		let filepath = self.path.join(filename);
		let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(filepath).await?;
		let locked_data = data.lock().await;
		let serialized_data = ron::ser::to_string(&*locked_data)?;
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
	use std::collections::HashMap;
	use super::*;
	#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)] 
	pub struct T<A, B, C>
	{
		data: E<A, B, C>,
	}
	#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)] 
	pub enum E<A, B, C>
	{
		A {v: Option<A>},
		B {v: Option<B>},
		C {v: Option<C>},
	}
	#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)] 
	pub enum EA
	{
		AA, AB, AC
	}
	#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)] 
	pub enum EB
	{
		BA, BB, BC
	}
	#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)] 
	pub enum EC
	{
		CA, CB, CC
	}
	#[tokio::test]
	async fn deserialize_or_value() -> Result<()>
	{
		let se = Serder::new("eule_test".to_string()).await?;
		let mut hm: HashMap<String, E<EA, EB, EC>> = HashMap::new();
		hm.insert("B".to_string(), E::B{v: Some(EB::BC)});
		let data: HashMap<String, E<EA, EB, EC>> = se.deserialize_or_value("in_no_err.ron".to_string(), hm).await?;
		se.serialize_and_save("out_no_err.ron".to_string(), data).await?;

		Ok(())
	}
	#[tokio::test]
	async fn deserialize_or_value_arc_mutex() -> Result<()>
	{
		let se = Serder::new("eule_test".to_string()).await?;
		let mut hm: HashMap<String, E<EA, EB, EC>> = HashMap::new();
		hm.insert("B".to_string(), E::B{v: Some(EB::BC)});
		let data: HashMap<String, E<EA, EB, EC>> = se.deserialize_or_value("in_arc_mutex.ron".to_string(), hm).await?;
		let data_arc_mutex = Arc::new(Mutex::new(&data));
		se.serialize_arc_mutex_and_save("out_arc_mutex.ron".to_string(), Arc::clone(&data_arc_mutex)).await?;
		Ok(())
	}
	#[tokio::test]
	async fn serialize_arc_mutex_and_save() -> Result<()>
	{
		let se = Serder::new("eule_test".to_string()).await?;
		let mut hm: HashMap<String, E<EA, EB, EC>> = HashMap::new();
		hm.insert("B".to_string(), E::B{v: Some(EB::BC)});
		let data_arc_mutex = Arc::new(Mutex::new(&hm));
		se.serialize_arc_mutex_and_save("out_arc_mutex_save.ron".to_string(), Arc::clone(&data_arc_mutex)).await?;
		Ok(())
	}
}
