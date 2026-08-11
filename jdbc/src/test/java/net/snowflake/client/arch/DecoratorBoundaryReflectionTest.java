package net.snowflake.client.arch;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;

import com.tngtech.archunit.core.domain.JavaClass;
import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import com.tngtech.archunit.core.importer.ImportOption;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.List;
import java.util.stream.Collectors;
import net.snowflake.client.internal.api.decorator.AbstractDecorator;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.Decorators;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

/**
 * Guards the self-propagating decorator boundary the "runtime exception carrier" refactor relies
 * on: a JDBC object handed back from another JDBC object must return <em>decorated</em>, or it
 * silently escapes the telemetry + exception-translation boundary. That property is behavioural, so
 * ArchUnit (structure-only) can't see it — hence a reflection test.
 *
 * <p>The impl tier wraps returns two ways: the {@link Decorators} factory (statement / result set /
 * connection) and direct {@code new Decorated*} construction (metadata, clob, prepared / callable,
 * pooled). These tests cover both mechanisms without a live connection:
 *
 * <ul>
 *   <li>{@link Decorators} is <em>complete</em>: every concrete raw impl that is a {@link
 *       Statement} or {@link ResultSet} is wrapped into an {@link AbstractDecorator}, so a newly
 *       added impl subtype can't fall through the {@code instanceof} ladder and leave the boundary
 *       raw;
 *   <li>{@link Decorators} is idempotent (an already-decorated object passes through) and
 *       null-safe;
 *   <li>every generated {@code Decorated*} is a well-formed decorator — an {@link
 *       AbstractDecorator} that implements the JDBC interface it stands in for — so the
 *       direct-construction return sites hand back a genuine wrapper.
 * </ul>
 */
class DecoratorBoundaryReflectionTest {

  private static final JavaClasses IMPL_CLASSES =
      new ClassFileImporter()
          .withImportOption(ImportOption.Predefined.DO_NOT_INCLUDE_TESTS)
          .importPackages("net.snowflake.client.internal.api.implementation");

  private static List<Class<?>> rawImplsAssignableTo(Class<?> iface) {
    return IMPL_CLASSES.stream()
        .filter(c -> c.isAssignableTo(iface))
        .filter(
            c ->
                !c.isInterface()
                    && !c.getModifiers()
                        .contains(com.tngtech.archunit.core.domain.JavaModifier.ABSTRACT))
        .filter(c -> !c.isAssignableTo(AbstractDecorator.class))
        .map(JavaClass::reflect)
        .collect(Collectors.toList());
  }

  @TestFactory
  List<DynamicTest> shouldDecorateEveryConcreteStatementImpl() {
    List<Class<?>> impls = rawImplsAssignableTo(Statement.class);
    assertFalse(impls.isEmpty(), "expected to discover at least one raw Statement impl");
    return impls.stream()
        .map(
            impl ->
                DynamicTest.dynamicTest(
                    impl.getSimpleName(),
                    () -> {
                      Statement raw = (Statement) mock(impl);
                      Statement decorated = Decorators.statement(raw, Telemetry.NOOP);
                      assertInstanceOf(
                          AbstractDecorator.class,
                          decorated,
                          impl.getName() + " must leave Decorators.statement decorated");
                    }))
        .collect(Collectors.toList());
  }

  @TestFactory
  List<DynamicTest> shouldDecorateEveryConcreteResultSetImpl() {
    List<Class<?>> impls = rawImplsAssignableTo(ResultSet.class);
    assertFalse(impls.isEmpty(), "expected to discover at least one raw ResultSet impl");
    return impls.stream()
        .map(
            impl ->
                DynamicTest.dynamicTest(
                    impl.getSimpleName(),
                    () -> {
                      ResultSet raw = (ResultSet) mock(impl);
                      ResultSet decorated = Decorators.resultSet(raw, Telemetry.NOOP);
                      assertInstanceOf(
                          AbstractDecorator.class,
                          decorated,
                          impl.getName() + " must leave Decorators.resultSet decorated");
                    }))
        .collect(Collectors.toList());
  }

  @Test
  void shouldTreatAlreadyDecoratedObjectsAsIdempotent() {
    Statement rawStmt = (Statement) mock(firstRawImpl(Statement.class));
    Statement once = Decorators.statement(rawStmt, Telemetry.NOOP);
    Statement twice = Decorators.statement(once, Telemetry.NOOP);
    assertSame(once, twice, "an already-decorated statement should pass through unchanged");

    ResultSet rawRs = (ResultSet) mock(firstRawImpl(ResultSet.class));
    ResultSet rsOnce = Decorators.resultSet(rawRs, Telemetry.NOOP);
    ResultSet rsTwice = Decorators.resultSet(rsOnce, Telemetry.NOOP);
    assertSame(rsOnce, rsTwice, "an already-decorated result set should pass through unchanged");
  }

  @Test
  void shouldBeNullSafe() {
    assertNull(Decorators.statement(null, Telemetry.NOOP));
    assertNull(Decorators.resultSet(null, Telemetry.NOOP));
  }

  @TestFactory
  List<DynamicTest> shouldMakeEveryGeneratedDecoratorAWellFormedWrapper() {
    List<JavaClass> decorators =
        IMPL_CLASSES.stream()
            .filter(c -> c.getSimpleName().startsWith("Decorated"))
            .filter(c -> !c.isInterface())
            .collect(Collectors.toList());
    assertFalse(decorators.isEmpty(), "expected to discover the generated Decorated* wrappers");
    return decorators.stream()
        .map(
            dec ->
                DynamicTest.dynamicTest(
                    dec.getSimpleName(),
                    () -> {
                      assertTrue(
                          dec.isAssignableTo(AbstractDecorator.class),
                          dec.getName() + " must extend AbstractDecorator");
                      assertTrue(
                          dec.getAllRawInterfaces().stream()
                              .anyMatch(
                                  i ->
                                      i.getPackageName().equals("java.sql")
                                          || i.getPackageName().equals("javax.sql")),
                          dec.getName() + " must implement the JDBC interface it decorates");
                    }))
        .collect(Collectors.toList());
  }

  private static Class<?> firstRawImpl(Class<?> iface) {
    return rawImplsAssignableTo(iface).stream()
        .findFirst()
        .orElseThrow(() -> new AssertionError("no raw impl found for " + iface.getName()));
  }
}
