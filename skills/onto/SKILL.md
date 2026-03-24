---
name: onto
description: "프로젝트 작업 지식 온톨로지를 조회, 추가, 검색, 검증한다. 프로젝트의 도메인 개념, 아키텍처, 워크플로우, 규약을 관리할 때 사용."
user-invocable: true
allowed-tools: mcp__onto__recall, mcp__onto__upsert, mcp__onto__delete, mcp__onto__search, mcp__onto__list, mcp__onto__graph, mcp__onto__reindex, mcp__onto__validate
argument-hint: "[recall|add|search|graph|validate|list] [query]"
---

프로젝트 작업 지식 온톨로지(scope: project)에 대해 요청된 작업을 수행한다.

## 사용 가능한 작업

- **recall [context]** — 컨텍스트에서 연관 노드를 연상 검색
- **add** — 새 노드 추가 (이름, 카테고리, 태그, 참조, 본문 필요)
- **search [query]** — 태그/내용/참조로 노드 검색
- **graph [node-name]** — 특정 노드의 연결 그래프 표시
- **validate** — 깨진 참조 검출
- **list [category]** — 노드 목록 (카테고리/태그 필터 가능)
- **reindex** — _index.md 재생성

## 노드 구조

각 노드는 markdown + YAML frontmatter:
- `name`: 고유 식별자
- `category`: domain, architecture, workflow, conventions
- `tags`: 태그 배열
- `refs`: 다른 노드 이름 배열
- `body`: 본문 (markdown, [[wikilink]] 사용 가능)

모든 MCP 도구 호출 시 `scope: "project"`를 사용한다.
