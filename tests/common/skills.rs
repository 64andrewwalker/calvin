use crate::common::TestEnv;

pub fn write_project_skill(
    env: &TestEnv,
    id: &str,
    skill_md: &str,
    supplementals: &[(&str, &str)],
) {
    env.write_project_file(&format!(".promptpack/skills/{}/SKILL.md", id), skill_md);
    for (rel_path, content) in supplementals {
        env.write_project_file(&format!(".promptpack/skills/{}/{}", id, rel_path), content);
    }
}
