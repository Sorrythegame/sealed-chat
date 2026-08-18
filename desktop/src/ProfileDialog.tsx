import { Camera, Check, LoaderCircle, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { ProfileInfo, ProfileUpdate, UserInfo } from "./api";
import { cropAvatar, DEFAULT_AVATAR } from "./avatar";

interface EditableProfileDialogProps {
  mode: "edit";
  profile: ProfileInfo;
  busy: boolean;
  error: string;
  onClose: () => void;
  onSave: (update: ProfileUpdate, nextProfile: ProfileInfo) => Promise<void>;
}

interface ReadonlyProfileDialogProps {
  mode: "view";
  profile: UserInfo;
  onClose: () => void;
}

type ProfileDialogProps = EditableProfileDialogProps | ReadonlyProfileDialogProps;

export default function ProfileDialog(props: ProfileDialogProps) {
  const { profile, onClose } = props;
  const [avatar, setAvatar] = useState(profile.avatar);
  const [bio, setBio] = useState(profile.bio || "");
  const [localError, setLocalError] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setAvatar(profile.avatar);
    setBio(profile.bio || "");
    setLocalError("");
  }, [profile]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !(props.mode === "edit" && props.busy)) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose, props]);

  async function chooseAvatar(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    setLocalError("");
    try {
      setAvatar(await cropAvatar(file));
    } catch (error) {
      setLocalError(String(error));
    } finally {
      if (fileRef.current) fileRef.current.value = "";
    }
  }

  async function save() {
    if (props.mode !== "edit") return;
    const normalizedBio = bio.trim();
    const update: ProfileUpdate = { bio: normalizedBio };
    if (avatar && avatar !== profile.avatar) update.avatar = avatar;
    await props.onSave(update, { ...profile, avatar, bio: normalizedBio });
  }

  const characterCount = Array.from(bio).length;
  const remoteError = props.mode === "edit" ? props.error : "";

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className={`profile-dialog ${props.mode === "view" ? "readonly" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={props.mode === "edit" ? "编辑个人资料" : `${profile.username}的个人资料`}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button className="modal-close" type="button" aria-label="关闭" onClick={onClose}>
          <X size={18} />
        </button>

        <div className="profile-dialog-heading">
          <div className="profile-avatar-wrap">
            <img src={avatar || DEFAULT_AVATAR} alt={`${profile.username}的头像`} />
            {props.mode === "edit" && (
              <button type="button" aria-label="更换头像" title="更换头像" onClick={() => fileRef.current?.click()}>
                <Camera size={17} />
              </button>
            )}
          </div>
          <div>
            <span className="eyebrow">{props.mode === "edit" ? "个人资料" : "联系人资料"}</span>
            <h2>{profile.username}</h2>
          </div>
        </div>

        {props.mode === "edit" ? (
          <>
            <label className="profile-field">
              <span>用户名</span>
              <input value={profile.username} disabled />
            </label>
            <label className="profile-field">
              <span>个人简介</span>
              <textarea
                value={bio}
                placeholder="写一句简短的自我介绍"
                onChange={(event) => {
                  const next = Array.from(event.target.value).slice(0, 120).join("");
                  setBio(next);
                }}
              />
              <small>{characterCount}/120</small>
            </label>
            {(localError || remoteError) && <div className="inline-error">{localError || remoteError}</div>}
            <button className="profile-save" type="button" disabled={props.busy} onClick={() => void save()}>
              {props.busy ? <LoaderCircle className="spin" size={17} /> : <Check size={17} />}
              {props.busy ? "保存中" : "保存资料"}
            </button>
            <input ref={fileRef} hidden type="file" accept="image/*" onChange={chooseAvatar} />
          </>
        ) : (
          <div className="readonly-bio">
            <span>个人简介</span>
            <p>{bio || "这个人很低调，还没有填写简介。"}</p>
          </div>
        )}
      </section>
    </div>
  );
}
