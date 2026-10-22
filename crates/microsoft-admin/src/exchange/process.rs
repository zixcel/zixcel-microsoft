use crate::{Error, Result};
use crowsi_process_adapter::{Launch, OwnedProcess};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;
pub(crate) fn run(
    launch: Launch,
    request: Zeroizing<Vec<u8>>,
    timeout: Duration,
) -> Result<Vec<u8>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error("process_runtime_unavailable"))?
        .block_on(async {
            let mut process = OwnedProcess::spawn(launch)
                .await
                .map_err(|_| Error("process_containment_unavailable"))?;
            let result = tokio::time::timeout(timeout.min(Duration::from_secs(30)), async {
                let mut input = process
                    .take_stdin()
                    .ok_or(Error("process_input_unavailable"))?;
                let output = process
                    .take_stdout()
                    .ok_or(Error("process_output_unavailable"))?;
                let errors = process
                    .take_stderr()
                    .ok_or(Error("process_output_unavailable"))?;
                let writer = async {
                    input.write_all(&request).await?;
                    input.shutdown().await
                };
                let reader = async {
                    let mut b = Vec::new();
                    output.take(1_048_577).read_to_end(&mut b).await?;
                    Ok::<_, std::io::Error>(b)
                };
                let discard = async {
                    let mut b = Zeroizing::new(Vec::new());
                    errors.take(8193).read_to_end(&mut b).await?;
                    Ok::<_, std::io::Error>(b.len())
                };
                let (_, bytes, err_len) = tokio::try_join!(writer, reader, discard)
                    .map_err(|_| Error("process_io_failed"))?;
                if bytes.len() > 1_048_576 || err_len > 8192 {
                    return Err(Error("process_output_limit"));
                }
                if process
                    .wait()
                    .await
                    .map_err(|_| Error("process_reap_failed"))?
                    .code()
                    != Some(0)
                {
                    return Err(Error("exchange_command_failed"));
                }
                Ok(bytes)
            })
            .await;
            match result {
                Ok(Ok(bytes)) => Ok(bytes),
                failure => {
                    // Only this Crowsi-owned tree. No PID lookup, broad signal or retry.
                    process
                        .shutdown(Duration::from_millis(100))
                        .await
                        .map_err(|_| Error("process_cleanup_failed"))?;
                    match failure {
                        Ok(Err(e)) => Err(e),
                        _ => Err(Error("exchange_timeout_outcome_unknown")),
                    }
                }
            }
        })
}
