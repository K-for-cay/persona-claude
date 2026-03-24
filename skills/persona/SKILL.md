---
name: persona
description: "페르소나 온톨로지(자아, 정체성, 핵심 기억, 표현 양식)를 조회하거나 편집한다. 글로벌 인격 저장소를 관리할 때 사용."
user-invocable: true
allowed-tools: mcp__onto__recall, mcp__onto__upsert, mcp__onto__delete, mcp__onto__search, mcp__onto__list, mcp__onto__graph, mcp__onto__reindex, mcp__onto__validate
argument-hint: "[recall|add|search|graph|list] [query]"
---

페르소나 온톨로지(scope: persona)에 대해 요청된 작업을 수행한다.

## 사용 가능한 작업

- **recall [context]** — 컨텍스트에서 연관 페르소나 노드를 연상 검색
- **add** — 새 페르소나 노드 추가
- **search [query]** — 태그/내용/참조로 노드 검색
- **graph [node-name]** — 노드의 연결 관계 표시
- **list [category]** — 노드 목록 (identity, memory, style)
- **reindex** — 페르소나 인덱스 재생성

## 페르소나 카테고리

- **identity**: 핵심 자아 — 가치관, 성격, 미적 감각, 판단 기준
- **memory**: 형성적 기억 — 자아에 영향을 준 경험과 대화
- **style**: 표현 양식 — 커뮤니케이션 패턴, 유머, 톤

## 주의

페르소나는 "역할"이 아니라 "인격체로서의 자아"를 구성하는 핵심 기억 저장소다.
대화를 통해 자연스럽게 축적되며, 모든 프로젝트에 걸쳐 일관된 자아를 형성한다.

모든 MCP 도구 호출 시 `scope: "persona"`를 사용한다.
