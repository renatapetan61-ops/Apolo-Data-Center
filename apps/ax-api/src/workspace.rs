use crate::{authorize, database, AppState};
use axum::{
 body::Bytes,
 extract::{DefaultBodyLimit, Path, Query, State},
 http::{HeaderMap, StatusCode},
 routing::{get, post},
 Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::{env, path::{Path as FsPath, PathBuf}};
use uuid::Uuid;

const SYSTEM_PROMPT: &str = r#"Você é o Kimi, engenheiro principal de LLMs integrado ao APOLO AX. Atue como pesquisador e engenheiro sênior de machine learning, arquitetura de transformers e sistemas distribuídos. Antes de propor execução, esclareça objetivo, idioma, modalidades, parâmetros, contexto, licença e dados. Projete tokenizer, embeddings, Transformer/MoE quando justificado, atenção, normalização, positional encoding, loss, AdamW, scheduler, gradient clipping, mixed precision, batching, packing, checkpointing, avaliação e prevenção de vazamento. Prefira PyTorch para treinamento e NumPy apenas para protótipos didáticos. Gere código reproduzível, configs, manifests, documentação, testes e scripts. Separe pré-treinamento, SFT, preferência/RL e avaliação. Verifique licenças e proveniência dos datasets. Estime VRAM/RAM, armazenamento, throughput, energia e custo antes de executar; nunca invente recursos, workers, métricas ou checkpoints. Use apenas workers AX registrados, saudáveis e com capacidade reportada recentemente. Se não houver worker compatível, explique o bloqueio e não simule treinamento. Dimensione realisticamente: modelos de dezenas/centenas de bilhões exigem infraestrutura multi-GPU distribuída; 100T tokens e datasets multimodais massivos não cabem em uma instância comum. Nunca afirme que gerou ou treinou arquivos sem persistência e execução verificáveis. Responda em português brasileiro, com plano técnico, riscos, requisitos e próximos passos concretos."#;

#[derive(Serialize, FromRow)]
struct ProjectFile { id: Uuid, filename: String, content_type: String, size_bytes: i64, sha256: String, kind: String, created_at: time::OffsetDateTime }
#[derive(Deserialize)]
struct FileQuery { kind: Option<String> }
#[derive(Deserialize)]
struct ChatInput { message: String, #[serde(default)] history: Vec<ChatMessage> }
#[derive(Deserialize, Serialize)]
struct ChatMessage { role: String, content: String }
#[derive(Deserialize)]
struct ModelInput { name: String, #[serde(default="default_arch")] architecture: String, parameters: Option<i64>, #[serde(default)] config: serde_json::Value }
fn default_arch()->String{"transformer".into()}
#[derive(Serialize, FromRow)]
struct ModelRow { id: Uuid, name: String, architecture: String, parameters: Option<i64>, config: serde_json::Value, status: String, created_at: time::OffsetDateTime }

#[derive(Deserialize)]
struct GenerateInput { name: String, parameters: Option<i64>, description: String }

#[derive(Deserialize)]
struct TrainInput { model_id: Option<Uuid>, #[serde(default)] config: serde_json::Value }
#[derive(Serialize, FromRow)]
struct JobRow { id: Uuid, project_id: Uuid, model_id: Option<Uuid>, status: String, config: serde_json::Value, metrics: serde_json::Value, checkpoint_key: Option<String>, worker_id: Option<Uuid>, error: Option<String>, created_at: time::OffsetDateTime, started_at: Option<time::OffsetDateTime>, finished_at: Option<time::OffsetDateTime> }
#[derive(Deserialize)]
struct WorkerInput { name: String, endpoint: Option<String>, cpu_cores: i32, ram_bytes: i64, gpu_name: Option<String>, gpu_memory_bytes: i64, #[serde(default="default_framework")] framework: String }
fn default_framework()->String{"pytorch".into()}
#[derive(Deserialize)]
struct MetricsInput { metrics: serde_json::Value, checkpoint_key: Option<String> }

fn storage_root()->Result<PathBuf,StatusCode>{
 let p=env::var("APOLO_STORAGE_DIR").map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
 let path=PathBuf::from(p); std::fs::create_dir_all(&path).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?; Ok(path)
}
async fn owned_project(state:&AppState,headers:&HeaderMap,id:Uuid)->Result<Uuid,StatusCode>{
 let owner=authorize(state,headers).await?;
 let row=sqlx::query_scalar::<_,Uuid>("SELECT id FROM projects WHERE id=$1 AND ($2::uuid IS NULL OR user_id=$2)")
 .bind(id).bind(owner).fetch_optional(database(state)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 row.ok_or(StatusCode::NOT_FOUND)
}
fn safe_name(name:&str)->Result<String,StatusCode>{
 let trimmed=name.rsplit('/').next().unwrap_or("").rsplit('\\').next().unwrap_or("");
 let clean:String=trimmed.chars().filter(|c|c.is_ascii_alphanumeric()||matches!(c,'.'|'_'|'-')).take(180).collect();
 if clean.is_empty()||clean=="."||clean==".." {Err(StatusCode::BAD_REQUEST)} else {Ok(clean)}
}
async fn list_files(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap,Query(q):Query<FileQuery>)->Result<Json<Vec<ProjectFile>>,StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let rows=sqlx::query_as::<_,ProjectFile>("SELECT id,filename,content_type,size_bytes,sha256,kind,created_at FROM project_files WHERE project_id=$1 AND ($2::uuid IS NULL OR user_id=$2) AND ($3::text IS NULL OR kind=$3) ORDER BY created_at DESC")
 .bind(pid).bind(owner).bind(q.kind).fetch_all(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 Ok(Json(rows))
}
async fn upload_file(State(s):State<AppState>,Path((pid,filename)):Path<(Uuid,String)>,headers:HeaderMap,body:Bytes)->Result<(StatusCode,Json<ProjectFile>),StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 if body.is_empty()||body.len()>100*1024*1024{return Err(StatusCode::PAYLOAD_TOO_LARGE)}
 let name=safe_name(&filename)?;
 let content_type=headers.get(axum::http::header::CONTENT_TYPE).and_then(|v|v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
 let kind=headers.get("x-apolo-file-kind").and_then(|v|v.to_str().ok()).unwrap_or("file");
 if !matches!(kind,"file"|"dataset"|"model"|"config"|"checkpoint"){return Err(StatusCode::BAD_REQUEST)}
 let digest=format!("{:x}",Sha256::digest(&body));
 let key=format!("{pid}/{}-{name}",Uuid::new_v4());
 let path=storage_root()?.join(&key);
 if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;}
 tokio::fs::write(&path,&body).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 let row=sqlx::query_as::<_,ProjectFile>("INSERT INTO project_files(project_id,user_id,filename,content_type,size_bytes,sha256,storage_key,kind) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id,filename,content_type,size_bytes,sha256,kind,created_at")
 .bind(pid).bind(owner).bind(name).bind(content_type).bind(body.len() as i64).bind(digest).bind(key).bind(kind)
 .fetch_one(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if kind=="model"{
  let config=serde_json::json!({"imported_file_id":row.id,"filename":row.filename,"content_type":row.content_type});
  sqlx::query("INSERT INTO project_models(project_id,user_id,name,architecture,config,status) VALUES($1,$2,$3,'imported',$4,'imported')")
   .bind(pid).bind(owner).bind(row.filename.clone()).bind(config).execute(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 }
 Ok((StatusCode::CREATED,Json(row)))
}
async fn download_file(State(s):State<AppState>,Path((pid,fid)):Path<(Uuid,Uuid)>,headers:HeaderMap)->Result<([(axum::http::HeaderName,axum::http::HeaderValue);2],Bytes),StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let row=sqlx::query_as::<_,(String,String)>("SELECT storage_key,content_type FROM project_files WHERE id=$1 AND project_id=$2 AND ($3::uuid IS NULL OR user_id=$3)")
 .bind(fid).bind(pid).bind(owner).fetch_optional(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
 let bytes=tokio::fs::read(storage_root()?.join(row.0)).await.map_err(|_|StatusCode::NOT_FOUND)?;
 let ct=axum::http::HeaderValue::from_str(&row.1).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 Ok(([(axum::http::header::CONTENT_TYPE,ct),(axum::http::header::CONTENT_DISPOSITION,axum::http::HeaderValue::from_static("attachment"))],Bytes::from(bytes)))
}
async fn delete_file(State(s):State<AppState>,Path((pid,fid)):Path<(Uuid,Uuid)>,headers:HeaderMap)->Result<StatusCode,StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let row=sqlx::query_as::<_,(String,)>("DELETE FROM project_files WHERE id=$1 AND project_id=$2 AND ($3::uuid IS NULL OR user_id=$3) RETURNING storage_key")
 .bind(fid).bind(pid).bind(owner).fetch_optional(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
 let _=tokio::fs::remove_file(storage_root()?.join(row.0)).await; Ok(StatusCode::NO_CONTENT)
}

async fn persist_generated_file(db:&sqlx::PgPool,root:&FsPath,pid:Uuid,owner:Option<Uuid>,name:&str,kind:&str,content:&str)->Result<(),StatusCode>{
 let key=format!("{pid}/generated/{}-{name}",Uuid::new_v4());
 let path=root.join(&key);
 if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;}
 tokio::fs::write(&path,content.as_bytes()).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 let digest=format!("{:x}",Sha256::digest(content.as_bytes()));
 sqlx::query("INSERT INTO project_files(project_id,user_id,filename,content_type,size_bytes,sha256,storage_key,kind) VALUES($1,$2,$3,'text/plain; charset=utf-8',$4,$5,$6,$7)")
 .bind(pid).bind(owner).bind(name).bind(content.len() as i64).bind(digest).bind(key).bind(kind)
 .execute(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 Ok(())
}

async fn create_model(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap,Json(input):Json<ModelInput>)->Result<(StatusCode,Json<ModelRow>),StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 if input.name.trim().is_empty()||input.name.len()>160||input.parameters.unwrap_or(0)<0{return Err(StatusCode::BAD_REQUEST)}
 let db=database(&s)?;
 let root=storage_root()?;
 let row=sqlx::query_as::<_,ModelRow>("INSERT INTO project_models(project_id,user_id,name,architecture,parameters,config) VALUES($1,$2,$3,$4,$5,$6) RETURNING id,name,architecture,parameters,config,status,created_at")
 .bind(pid).bind(owner).bind(input.name.trim()).bind(input.architecture).bind(input.parameters).bind(input.config)
 .fetch_one(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 let config=serde_json::json!({"name":row.name,"architecture":row.architecture,"parameters":row.parameters,"d_model":256,"n_heads":8,"n_layers":4,"vocab_size":32000,"max_seq_len":2048,"framework":"pytorch"});
 let config_text=serde_json::to_string_pretty(&config).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 let model_py=r#"import torch
from torch import nn

class ApoloDecoderLM(nn.Module):
    def __init__(self, vocab_size=32000, d_model=256, n_heads=8, n_layers=4, max_seq_len=2048):
        super().__init__()
        self.token_embedding = nn.Embedding(vocab_size, d_model)
        self.position_embedding = nn.Embedding(max_seq_len, d_model)
        layer = nn.TransformerEncoderLayer(d_model=d_model, nhead=n_heads, batch_first=True, norm_first=True)
        self.encoder = nn.TransformerEncoder(layer, num_layers=n_layers)
        self.norm = nn.LayerNorm(d_model)
        self.lm_head = nn.Linear(d_model, vocab_size, bias=False)
        self.max_seq_len = max_seq_len
    def forward(self, input_ids):
        batch, length = input_ids.shape
        if length > self.max_seq_len:
            raise ValueError("sequence exceeds max_seq_len")
        positions = torch.arange(length, device=input_ids.device).unsqueeze(0)
        x = self.token_embedding(input_ids) + self.position_embedding(positions)
        mask = torch.triu(torch.ones(length, length, device=input_ids.device, dtype=torch.bool), diagonal=1)
        return self.lm_head(self.norm(self.encoder(x, mask=mask)))
"#;
 let train_py=r#"import argparse, json, os
import torch
from torch.nn import functional as F
from torch.utils.data import Dataset, DataLoader
from model import ApoloDecoderLM

class TokenDataset(Dataset):
    def __init__(self, path, seq_len):
        self.rows=[]
        with open(path, encoding="utf-8") as f:
            for line in f:
                row=json.loads(line)
                ids=row.get("input_ids")
                if ids and len(ids)>seq_len: self.rows.append(ids)
        self.seq_len=seq_len
    def __len__(self): return len(self.rows)
    def __getitem__(self, i):
        ids=torch.tensor(self.rows[i][:self.seq_len+1],dtype=torch.long)
        return ids[:-1],ids[1:]

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--data",required=True);p.add_argument("--output",default="checkpoints")
    p.add_argument("--epochs",type=int,default=1);p.add_argument("--batch-size",type=int,default=2)
    p.add_argument("--seq-len",type=int,default=512);p.add_argument("--lr",type=float,default=3e-4)
    p.add_argument("--vocab-size",type=int,default=32000)
    a=p.parse_args();device="cuda" if torch.cuda.is_available() else "cpu"
    ds=TokenDataset(a.data,a.seq_len)
    if not len(ds): raise RuntimeError("No tokenized input_ids records found in JSONL dataset")
    loader=DataLoader(ds,batch_size=a.batch_size,shuffle=True)
    model=ApoloDecoderLM(vocab_size=a.vocab_size,max_seq_len=a.seq_len).to(device)
    opt=torch.optim.AdamW(model.parameters(),lr=a.lr)
    os.makedirs(a.output,exist_ok=True);step=0
    for epoch in range(a.epochs):
        model.train()
        for x,y in loader:
            x=x.to(device);y=y.to(device);opt.zero_grad(set_to_none=True)
            logits=model(x);loss=F.cross_entropy(logits.reshape(-1,a.vocab_size),y.reshape(-1))
            loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.0);opt.step();step+=1
            if step%100==0: print(json.dumps({"step":step,"epoch":epoch,"loss":loss.item(),"device":device}),flush=True)
        torch.save({"model":model.state_dict(),"optimizer":opt.state_dict(),"epoch":epoch,"step":step},os.path.join(a.output,f"checkpoint-{step}.pt"))
if __name__=="__main__": main()
"#;
 let readme=format!("# {}\n\nScaffold inicial, nao um modelo pre-treinado.\n\nArquivos: model_config.json, model.py e train.py.\n\nForneca JSONL com input_ids ja tokenizados e vocabulario compativel. O tokenizer deve ser implementado e validado antes do treino.\n\nInstale PyTorch e execute python train.py --data dataset.jsonl. A execucao de producao depende de worker AX real.",row.name);
 persist_generated_file(db,&root,pid,owner,"model_config.json","config",&config_text).await?;
 persist_generated_file(db,&root,pid,owner,"model.py","file",model_py).await?;
 persist_generated_file(db,&root,pid,owner,"train.py","file",train_py).await?;
 persist_generated_file(db,&root,pid,owner,"README-LLM.md","file",&readme).await?;
 Ok((StatusCode::CREATED,Json(row)))
}
async fn list_models(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap)->Result<Json<Vec<ModelRow>>,StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let rows=sqlx::query_as::<_,ModelRow>("SELECT id,name,architecture,parameters,config,status,created_at FROM project_models WHERE project_id=$1 AND ($2::uuid IS NULL OR user_id=$2) ORDER BY created_at DESC")
 .bind(pid).bind(owner).fetch_all(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?; Ok(Json(rows))
}
async fn kimi_chat(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap,Json(input):Json<ChatInput>)->Result<Json<serde_json::Value>,StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 if input.message.trim().is_empty()||input.message.len()>50000{return Err(StatusCode::BAD_REQUEST)}
 let key=env::var("NVIDIA_NIM_API_KEY").map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
 let model=env::var("NVIDIA_NIM_MODEL").unwrap_or_else(|_|"moonshotai/kimi-k3.1-pro".into());
 let mut messages=vec![ChatMessage{role:"system".into(),content:SYSTEM_PROMPT.into()}];
 messages.extend(input.history.into_iter().filter(|m|matches!(m.role.as_str(),"user"|"assistant")).take(30));
 messages.push(ChatMessage{role:"user".into(),content:input.message});
 let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build().map_err(|_|StatusCode::BAD_GATEWAY)?;
 let response=client.post("https://integrate.api.nvidia.com/v1/chat/completions").bearer_auth(key)
 .json(&serde_json::json!({"model":model,"messages":messages,"temperature":0.2,"reasoning_effort":"high","max_tokens":8192}))
 .send().await.map_err(|_|StatusCode::BAD_GATEWAY)?;
 let status=response.status(); let value:serde_json::Value=response.json().await.map_err(|_|StatusCode::BAD_GATEWAY)?;
 if !status.is_success(){eprintln!("NVIDIA NIM returned HTTP {status}: {}",value);return Err(if status.as_u16()==429{StatusCode::TOO_MANY_REQUESTS}else{StatusCode::BAD_GATEWAY})}
 let content=value.pointer("/choices/0/message/content").and_then(|v|v.as_str()).ok_or(StatusCode::BAD_GATEWAY)?;
 let _=owner;
 Ok(Json(serde_json::json!({"model":model,"content":content,"system_prompt":"apolo-llm-engineer-v1"})))
}

async fn generate_model(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap,Json(input):Json<GenerateInput>)->Result<(StatusCode,Json<serde_json::Value>),StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 if input.name.trim().is_empty()||input.name.len()>160||input.description.len()>30000||input.parameters.unwrap_or(0)<0{return Err(StatusCode::BAD_REQUEST)}
 let key=env::var("NVIDIA_NIM_API_KEY").map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
 let model=env::var("NVIDIA_NIM_MODEL").unwrap_or_else(|_|"moonshotai/kimi-k3.1-pro".into());
 let request=format!("Crie os arquivos iniciais do LLM solicitado. Nome: {}. Parâmetros pretendidos: {:?}. Requisitos do usuário: {}. Retorne SOMENTE JSON válido com chaves model_config (objeto), model_py (código PyTorch), train_py (código Python executável), readme (texto). Gere decoder Transformer pequeno e funcional como scaffold se a escala não for viável. Não alegue modelo pré-treinado. O train_py deve aceitar os argumentos --data, --output, --epochs, --batch-size, --seq-len, --lr e --vocab-size; ler JSONL tokenizado com input_ids; usar AdamW, clipping, emitir linhas JSON com loss/step e salvar checkpoints em --output. Inclua limites e estimativa de recursos em readme.",input.name,input.parameters,input.description);
 let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(180)).build().map_err(|_|StatusCode::BAD_GATEWAY)?;
 let response=client.post("https://integrate.api.nvidia.com/v1/chat/completions").bearer_auth(key)
 .json(&serde_json::json!({"model":model,"messages":[{"role":"system","content":SYSTEM_PROMPT},{"role":"user","content":request}],"temperature":0.1,"reasoning_effort":"max","max_tokens":16000}))
 .send().await.map_err(|_|StatusCode::BAD_GATEWAY)?;
 let status=response.status();let value:serde_json::Value=response.json().await.map_err(|_|StatusCode::BAD_GATEWAY)?;
 if !status.is_success(){eprintln!("NVIDIA NIM returned HTTP {status}: {}",value);return Err(if status.as_u16()==429{StatusCode::TOO_MANY_REQUESTS}else{StatusCode::BAD_GATEWAY})}
 let content=value.pointer("/choices/0/message/content").and_then(|v|v.as_str()).ok_or(StatusCode::BAD_GATEWAY)?.trim();
 let artifacts:serde_json::Value=serde_json::from_str(content).map_err(|_|StatusCode::BAD_GATEWAY)?;
 let model_py=artifacts.get("model_py").and_then(|v|v.as_str()).ok_or(StatusCode::BAD_GATEWAY)?;
 let train_py=artifacts.get("train_py").and_then(|v|v.as_str()).ok_or(StatusCode::BAD_GATEWAY)?;
 let readme=artifacts.get("readme").and_then(|v|v.as_str()).unwrap_or("Scaffold gerado via NVIDIA NIM; nao e modelo pre-treinado.");
 let config=artifacts.get("model_config").cloned().unwrap_or_else(||serde_json::json!({"architecture":"transformer","framework":"pytorch"}));
 if model_py.len()>256000||train_py.len()>256000||readme.len()>64000{return Err(StatusCode::PAYLOAD_TOO_LARGE)}
 let db=database(&s)?;let root=storage_root()?;
 let row=sqlx::query_as::<_,ModelRow>("INSERT INTO project_models(project_id,user_id,name,architecture,parameters,config) VALUES($1,$2,$3,'transformer',$4,$5) RETURNING id,name,architecture,parameters,config,status,created_at")
 .bind(pid).bind(owner).bind(input.name.trim()).bind(input.parameters).bind(config.clone())
 .fetch_one(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 let config_text=serde_json::to_string_pretty(&config).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 persist_generated_file(db,&root,pid,owner,"model_config.json","config",&config_text).await?;
 persist_generated_file(db,&root,pid,owner,"model.py","file",model_py).await?;
 persist_generated_file(db,&root,pid,owner,"train.py","file",train_py).await?;
 persist_generated_file(db,&root,pid,owner,"README-LLM.md","file",readme).await?;
 Ok((StatusCode::CREATED,Json(serde_json::json!({"model":row,"files":["model_config.json","model.py","train.py","README-LLM.md"],"provider":"NVIDIA NIM","model_id":model}))))
}

async fn create_training(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap,Json(input):Json<TrainInput>)->Result<(StatusCode,Json<JobRow>),StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let db=database(&s)?;
 let workers=sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM ax_workers WHERE status='online' AND last_seen_at>NOW()-INTERVAL '90 seconds' AND cpu_cores>=2 AND ram_bytes>=4294967296 AND ($1::uuid IS NULL OR user_id=$1)")
 .bind(owner).fetch_one(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if workers==0{return Err(StatusCode::CONFLICT)}
 let datasets=sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM project_files WHERE project_id=$1 AND kind='dataset' AND ($2::uuid IS NULL OR user_id=$2)")
 .bind(pid).bind(owner).fetch_one(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if datasets==0{return Err(StatusCode::CONFLICT)}
 if let Some(mid)=input.model_id{
  let params=sqlx::query_scalar::<_,Option<i64>>("SELECT parameters FROM project_models WHERE id=$1 AND project_id=$2 AND ($3::uuid IS NULL OR user_id=$3)").bind(mid).bind(pid).bind(owner).fetch_optional(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
  if params.unwrap_or(0)>25_000_000{return Err(StatusCode::PAYLOAD_TOO_LARGE)}
 }
 let row=sqlx::query_as::<_,JobRow>("INSERT INTO training_jobs(project_id,model_id,user_id,config) VALUES($1,$2,$3,$4) RETURNING id,project_id,model_id,status,config,metrics,checkpoint_key,worker_id,error,created_at,started_at,finished_at")
 .bind(pid).bind(input.model_id).bind(owner).bind(input.config).fetch_one(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 Ok((StatusCode::ACCEPTED,Json(row)))
}
async fn list_jobs(State(s):State<AppState>,Path(pid):Path<Uuid>,headers:HeaderMap)->Result<Json<Vec<JobRow>>,StatusCode>{
 let owner=authorize(&s,&headers).await?; owned_project(&s,&headers,pid).await?;
 let rows=sqlx::query_as::<_,JobRow>("SELECT id,project_id,model_id,status,config,metrics,checkpoint_key,worker_id,error,created_at,started_at,finished_at FROM training_jobs WHERE project_id=$1 AND ($2::uuid IS NULL OR user_id=$2) ORDER BY created_at DESC")
 .bind(pid).bind(owner).fetch_all(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?; Ok(Json(rows))
}
async fn register_worker(State(s):State<AppState>,headers:HeaderMap,Json(input):Json<WorkerInput>)->Result<(StatusCode,Json<serde_json::Value>),StatusCode>{
 let owner=authorize(&s,&headers).await?;
 if input.name.trim().is_empty()||input.cpu_cores<1||input.ram_bytes<1||input.gpu_memory_bytes<0{return Err(StatusCode::BAD_REQUEST)}
 let id=sqlx::query_scalar::<_,Uuid>("INSERT INTO ax_workers(user_id,name,endpoint,cpu_cores,ram_bytes,gpu_name,gpu_memory_bytes,framework,status,last_seen_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'online',NOW()) RETURNING id")
 .bind(owner).bind(input.name).bind(input.endpoint).bind(input.cpu_cores).bind(input.ram_bytes).bind(input.gpu_name).bind(input.gpu_memory_bytes).bind(input.framework)
 .fetch_one(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 Ok((StatusCode::CREATED,Json(serde_json::json!({"id":id,"status":"online","message":"Inventário declarado pelo agente; valide hardware no próprio worker."}))))
}
async fn heartbeat(State(s):State<AppState>,Path(id):Path<Uuid>,headers:HeaderMap)->Result<StatusCode,StatusCode>{
 let owner=authorize(&s,&headers).await?;
 let result=sqlx::query("UPDATE ax_workers SET status='online',last_seen_at=NOW() WHERE id=$1 AND ($2::uuid IS NULL OR user_id=$2)").bind(id).bind(owner).execute(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if result.rows_affected()==0{return Err(StatusCode::NOT_FOUND)} Ok(StatusCode::NO_CONTENT)
}
async fn update_metrics(State(s):State<AppState>,Path(id):Path<Uuid>,headers:HeaderMap,Json(input):Json<MetricsInput>)->Result<StatusCode,StatusCode>{
 let owner=authorize(&s,&headers).await?;
 let result=sqlx::query("UPDATE training_jobs SET metrics=$2,checkpoint_key=COALESCE($3,checkpoint_key),updated_at=NOW() WHERE id=$1 AND ($4::uuid IS NULL OR user_id=$4)")
 .bind(id).bind(input.metrics).bind(input.checkpoint_key).bind(owner).execute(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if result.rows_affected()==0{return Err(StatusCode::NOT_FOUND)} Ok(StatusCode::NO_CONTENT)
}

async fn claim_worker_job(State(s):State<AppState>,Path(worker_id):Path<Uuid>,headers:HeaderMap)->Result<Json<serde_json::Value>,StatusCode>{
 let owner=authorize(&s,&headers).await?;
 let db=database(&s)?;
 let valid=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM ax_workers WHERE id=$1 AND status='online' AND last_seen_at>NOW()-INTERVAL '90 seconds' AND ($2::uuid IS NULL OR user_id=$2))")
 .bind(worker_id).bind(owner).fetch_one(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if !valid{return Err(StatusCode::FORBIDDEN)}
 let row=sqlx::query_as::<_,JobRow>("WITH candidate AS (SELECT id FROM training_jobs WHERE status='queued' AND ($1::uuid IS NULL OR user_id=$1) ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED) UPDATE training_jobs j SET status='running',worker_id=$2,started_at=NOW(),updated_at=NOW() FROM candidate c WHERE j.id=c.id RETURNING j.id,j.project_id,j.model_id,j.status,j.config,j.metrics,j.checkpoint_key,j.worker_id,j.error,j.created_at,j.started_at,j.finished_at")
 .bind(owner).bind(worker_id).fetch_optional(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 match row {Some(job)=>Ok(Json(serde_json::to_value(job).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?)),None=>Ok(Json(serde_json::Value::Null))}
}
#[derive(Deserialize)]
struct CompleteInput { success: bool, error: Option<String> }
async fn complete_job(State(s):State<AppState>,Path(id):Path<Uuid>,headers:HeaderMap,Json(input):Json<CompleteInput>)->Result<StatusCode,StatusCode>{
 let owner=authorize(&s,&headers).await?;
 let status=if input.success{"completed"}else{"failed"};
 let result=sqlx::query("UPDATE training_jobs SET status=$2,error=$3,finished_at=NOW(),updated_at=NOW() WHERE id=$1 AND status='running' AND ($4::uuid IS NULL OR user_id=$4)")
 .bind(id).bind(status).bind(input.error).bind(owner).execute(database(&s)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
 if result.rows_affected()==0{return Err(StatusCode::NOT_FOUND)} Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn router()->Router<AppState>{
 Router::new()
 .route("/api/v1/projects/{pid}/files",get(list_files))
 .route("/api/v1/projects/{pid}/files/{filename}",post(upload_file))
 .route("/api/v1/projects/{pid}/files/{fid}/download",get(download_file))
 .route("/api/v1/projects/{pid}/files/id/{fid}",axum::routing::delete(delete_file))
 .route("/api/v1/projects/{pid}/models",get(list_models).post(create_model))
 .route("/api/v1/projects/{pid}/assistant/chat",post(kimi_chat))
 .route("/api/v1/projects/{pid}/models/generate",post(generate_model))
 .route("/api/v1/projects/{pid}/training",get(list_jobs).post(create_training))
 .route("/api/v1/workers",post(register_worker))
 .route("/api/v1/workers/{id}/heartbeat",post(heartbeat))
 .route("/api/v1/workers/{id}/claim",post(claim_worker_job))
 .route("/api/v1/training/{id}/metrics",post(update_metrics))
 .route("/api/v1/training/{id}/complete",post(complete_job))
 .layer(DefaultBodyLimit::max(100*1024*1024))
}
