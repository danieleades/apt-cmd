use as_result::IntoResult;
use async_process::{Child, ChildStdout, Command};
use async_stream::stream;
use futures::io::BufReader;
use futures::prelude::*;
use futures::stream::StreamExt;
use futures_util::pin_mut;
use std::{io, pin::Pin};

#[derive(AsMut, Deref, DerefMut)]
#[as_mut(forward)]
pub struct Dpkg(Command);

impl Default for Dpkg {
    fn default() -> Self {
        let mut cmd = Command::new("dpkg");
        cmd.env("LANG", "C");
        Self(cmd)
    }
}

impl Dpkg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure_all(mut self) -> Self {
        self.args(&["--configure", "-a"]);
        self
    }

    pub async fn status(mut self) -> io::Result<()> {
        self.0.status().await?.into_result()
    }
}

pub type InstalledEvent = Pin<Box<dyn Stream<Item = String>>>;

#[derive(AsMut, Deref, DerefMut)]
#[as_mut(forward)]
pub struct DpkgQuery(Command);

impl Default for DpkgQuery {
    fn default() -> Self {
        let mut cmd = Command::new("dpkg-query");
        cmd.env("LANG", "C");
        Self(cmd)
    }
}

impl DpkgQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn show_installed<I, S>(mut self, packages: I) -> io::Result<(Child, InstalledEvent)>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.args(&["--show", "--showformat=${Package} ${db:Status-Status}\n"]);
        self.args(packages);

        let (child, stdout) = self.spawn_with_stdout().await?;

        let stdout = BufReader::new(stdout).lines();

        let stream = stream! {
            pin_mut!(stdout);
            while let Some(Ok(line)) = stdout.next().await {
                let mut fields = line.split(' ');
                let package = fields.next().unwrap();
                if fields.next().unwrap() == "installed" {
                    yield package.into();
                }
            }
        };

        Ok((child, Box::pin(stream)))
    }

    pub async fn status(mut self) -> io::Result<()> {
        self.0.status().await?.into_result()
    }

    pub async fn spawn_with_stdout(self) -> io::Result<(Child, ChildStdout)> {
        crate::utils::spawn_with_stdout(self.0).await
    }
}
